//! `ScannerService`: ponto único de orquestração de verificações, chamado
//! pelos comandos Tauri. Mantém o estado das tarefas ativas (para suportar
//! cancelamento) e delega classificação/política a `corvus-policy` e
//! execução de mecanismo a `corvus-clamav`.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex as StdMutex};
use std::time::Duration;

use corvus_clamav::ScanEngineAdapter;
use corvus_core::{
    EngineHealth, QuarantineItem, QuarantineItemId, ScanJob, ScanJobId, ScanKind, ScanResult,
    ScanResultId, ScanStatus, Verdict, VerdictCounts,
};
use corvus_policy::{
    classify_entry, full_scan_targets, quick_scan_targets, validate_custom_targets,
    FullScanAvailability,
};
use corvus_quarantine::QuarantineManager;
use corvus_storage::Database;
use tokio::sync::{mpsc, watch};

use crate::error::ScannerError;
use crate::events::ScanEvent;

/// Tempo máximo de uma única verificação antes de ser abortada
/// automaticamente. Evita que um alvo problemático (ex.: montagem de rede
/// lenta) prenda a tarefa indefinidamente.
const SCAN_TIMEOUT: Duration = Duration::from_secs(60 * 30);

const ENGINE_NAME: &str = "ClamAV";

pub struct ScannerService {
    db: Arc<Database>,
    clamav: Option<Arc<ScanEngineAdapter>>,
    active_jobs: Arc<StdMutex<HashMap<ScanJobId, watch::Sender<bool>>>>,
    /// `None` apenas se o diretório de quarentena não pôde ser preparado
    /// (ver `QuarantineManager::new`) — nesse caso os comandos de
    /// quarentena retornam `ScannerError::QuarantineUnavailable` em vez de
    /// fingir que a função existe.
    quarantine: Option<Arc<QuarantineManager>>,
}

impl ScannerService {
    pub fn new(db: Arc<Database>, clamav: Option<ScanEngineAdapter>) -> Self {
        let quarantine = match QuarantineManager::new(db.clone()) {
            Ok(manager) => Some(Arc::new(manager)),
            Err(error) => {
                tracing::error!(
                    %error,
                    "não foi possível preparar o diretório de quarentena; a função de quarentena ficará indisponível nesta sessão"
                );
                None
            }
        };
        Self {
            db,
            clamav: clamav.map(Arc::new),
            active_jobs: Arc::new(StdMutex::new(HashMap::new())),
            quarantine,
        }
    }

    /// Saúde atual do mecanismo de varredura (nunca reporta `Available` sem
    /// ter sondado o executável real).
    pub async fn engine_health(&self) -> EngineHealth {
        let executable = self
            .clamav
            .as_ref()
            .map(|a| a.executable_path().to_path_buf());
        let result =
            tokio::task::spawn_blocking(move || corvus_clamav::probe_health(executable.as_deref()))
                .await;

        match result {
            Ok(health) => health,
            Err(join_error) => {
                tracing::error!(error = %join_error, "falha ao sondar o mecanismo ClamAV");
                corvus_clamav::probe_health(None)
            }
        }
    }

    pub async fn recent_jobs(&self, limit: u32) -> Result<Vec<ScanJob>, ScannerError> {
        let db = self.db.clone();
        let jobs = tokio::task::spawn_blocking(move || db.list_recent_jobs(limit)).await??;
        Ok(jobs)
    }

    pub async fn job_results(&self, job_id: ScanJobId) -> Result<Vec<ScanResult>, ScannerError> {
        let db = self.db.clone();
        let results =
            tokio::task::spawn_blocking(move || db.list_results_for_job(job_id)).await??;
        Ok(results)
    }

    /// Contagem de vereditos por tarefa (ver `VerdictCounts`), usada pela
    /// interface para dar visibilidade rápida da qualidade de cada
    /// verificação no histórico sem precisar expandir os detalhes.
    pub async fn verdict_summaries(
        &self,
        job_ids: Vec<ScanJobId>,
    ) -> Result<HashMap<ScanJobId, VerdictCounts>, ScannerError> {
        let db = self.db.clone();
        let summaries =
            tokio::task::spawn_blocking(move || db.verdict_counts_for_jobs(&job_ids)).await??;
        Ok(summaries)
    }

    /// Remove do histórico as tarefas finalizadas com falha ou cancelamento.
    /// Deliberadamente não inclui estados não terminais: uma tarefa
    /// realmente em execução nunca pode ser apagada por este comando (ver
    /// `Database::reconcile_interrupted_jobs`, chamada na inicialização,
    /// que já converte tarefas presas de sessões anteriores em `Failed`
    /// antes que o usuário chegue a vê-las como "Verificando" indefinidamente).
    pub async fn clear_history(&self) -> Result<u64, ScannerError> {
        let db = self.db.clone();
        let removed = tokio::task::spawn_blocking(move || {
            db.delete_scan_jobs_by_status(&[ScanStatus::Failed, ScanStatus::Cancelled])
        })
        .await??;
        Ok(removed)
    }

    const SCAN_EXCLUSIONS_SETTING_KEY: &str = "scan_exclusions";

    /// Pastas que o scanner deve pular durante qualquer verificação
    /// (rápida, completa ou personalizada). Lista vazia por padrão.
    pub async fn get_scan_exclusions(&self) -> Result<Vec<String>, ScannerError> {
        let db = self.db.clone();
        let raw =
            tokio::task::spawn_blocking(move || db.get_setting(Self::SCAN_EXCLUSIONS_SETTING_KEY))
                .await??;
        match raw {
            Some(json) => Ok(serde_json::from_str(&json).unwrap_or_default()),
            None => Ok(Vec::new()),
        }
    }

    /// Substitui a lista de pastas excluídas das verificações. Caminhos são
    /// armazenados como informados (sem normalização adicional); a
    /// correspondência durante a varredura é por igualdade literal de
    /// caminho (o adaptador ClamAV escapa o caminho antes de repassá-lo
    /// como `--exclude-dir`), então um caminho e seus equivalentes com
    /// maiúsculas/minúsculas ou separadores diferentes não são
    /// automaticamente unificados nesta versão.
    pub async fn set_scan_exclusions(&self, paths: Vec<String>) -> Result<(), ScannerError> {
        let db = self.db.clone();
        let json = serde_json::to_string(&paths)?;
        tokio::task::spawn_blocking(move || {
            db.set_setting(Self::SCAN_EXCLUSIONS_SETTING_KEY, &json)
        })
        .await??;
        Ok(())
    }

    /// Move o arquivo de um resultado de verificação para a quarentena.
    ///
    /// Busca o resultado real no banco a partir de `scan_result_id` — nunca
    /// aceita um caminho de arquivo vindo direto da interface sem essa
    /// validação (evitaria que a UI mandasse isolar um caminho arbitrário).
    /// Não impõe aqui que o veredito seja `malicious`/`suspicious`: a
    /// decisão de isolar um arquivo com outro veredito é do usuário, e a
    /// interface já o orienta a fazer isso só quando há uma ameaça real.
    pub async fn quarantine_scan_result(
        &self,
        scan_result_id: ScanResultId,
    ) -> Result<QuarantineItem, ScannerError> {
        let Some(quarantine) = self.quarantine.clone() else {
            return Err(ScannerError::QuarantineUnavailable);
        };

        let db = self.db.clone();
        let result = tokio::task::spawn_blocking(move || db.get_scan_result(scan_result_id))
            .await??
            .ok_or(ScannerError::ScanResultNotFound(scan_result_id))?;

        let verdict_label = match result.verdict {
            Verdict::Malicious => "Ameaça encontrada",
            Verdict::Suspicious => "Item suspeito",
            Verdict::Unknown => "Resultado indeterminado",
            Verdict::Error => "Erro ao examinar",
            Verdict::Clean => "Nenhuma ameaça encontrada",
        };
        let detail = result
            .detections
            .first()
            .map(|d| format!(" ({})", d.signature_id))
            .unwrap_or_default();
        let reason = format!("{verdict_label}{detail} — isolado manualmente pelo usuário.");
        let path = PathBuf::from(&result.file_path);

        tokio::task::spawn_blocking(move || {
            quarantine.quarantine_file(&path, reason, Some(scan_result_id))
        })
        .await?
        .map_err(ScannerError::from)
    }

    pub async fn list_quarantine_items(&self) -> Result<Vec<QuarantineItem>, ScannerError> {
        let Some(quarantine) = self.quarantine.clone() else {
            return Err(ScannerError::QuarantineUnavailable);
        };
        Ok(tokio::task::spawn_blocking(move || quarantine.list_items()).await??)
    }

    /// Restaura um item da quarentena para o caminho original. Recusa a
    /// operação (sem mover nada) se o destino já existir ou se o hash do
    /// arquivo isolado não bater mais com o registrado — ver
    /// `QuarantineManager::restore_item`.
    pub async fn restore_quarantine_item(
        &self,
        item_id: QuarantineItemId,
    ) -> Result<QuarantineItem, ScannerError> {
        let Some(quarantine) = self.quarantine.clone() else {
            return Err(ScannerError::QuarantineUnavailable);
        };
        Ok(tokio::task::spawn_blocking(move || quarantine.restore_item(item_id)).await??)
    }

    /// Executa o `freshclam` manualmente, para atualizar a base de
    /// assinaturas sob pedido explícito do usuário (nunca automático ou
    /// agendado nesta versão — ver `docs/roadmap`). Roda sem privilégios
    /// elevados, exatamente como o usuário faria executando o binário
    /// diretamente; em instalações do ClamAV dentro de "Program Files"
    /// isso tende a falhar por falta de permissão de escrita, e nesse caso
    /// o erro real relatado pelo próprio `freshclam` é repassado à
    /// interface (ver `FreshclamOutcome`), nunca escondido ou interpretado
    /// como sucesso.
    pub async fn update_signatures(&self) -> Result<corvus_clamav::FreshclamOutcome, ScannerError> {
        Ok(corvus_clamav::run_freshclam().await?)
    }

    /// Solicita o cancelamento cooperativo de uma tarefa em execução.
    /// Retorna erro se a tarefa não estiver mais ativa (já concluída,
    /// cancelada ou com falha).
    pub fn cancel_scan(&self, job_id: ScanJobId) -> Result<(), ScannerError> {
        let jobs = self
            .active_jobs
            .lock()
            .expect("active_jobs mutex envenenado");
        match jobs.get(&job_id) {
            Some(tx) => {
                let _ = tx.send(true);
                Ok(())
            }
            None => Err(ScannerError::JobNotActive(job_id)),
        }
    }

    /// Inicia uma nova verificação de forma assíncrona: persiste a tarefa,
    /// devolve seu ID imediatamente e continua a execução em segundo plano,
    /// emitindo [`ScanEvent`]s em `events` conforme progride. Nunca bloqueia
    /// a thread chamadora durante a varredura em si.
    pub async fn start_scan(
        &self,
        kind: ScanKind,
        custom_targets: Option<Vec<PathBuf>>,
        events: mpsc::UnboundedSender<ScanEvent>,
    ) -> Result<ScanJobId, ScannerError> {
        let targets: Vec<PathBuf> = match kind {
            ScanKind::Quick => quick_scan_targets(),
            ScanKind::Full => match full_scan_targets() {
                FullScanAvailability::Available(paths) => paths,
                FullScanAvailability::Unavailable { reason } => {
                    return Err(ScannerError::ScanKindUnavailable(reason))
                }
            },
            ScanKind::Custom => {
                let requested = custom_targets.ok_or(ScannerError::MissingCustomTargets)?;
                validate_custom_targets(&requested)?
            }
        };

        let target_strings: Vec<String> = targets.iter().map(|p| p.display().to_string()).collect();
        let mut job = ScanJob::new(kind, target_strings);
        job.files_total = None; // desconhecido até o clamscan concluir; ver docs/roadmap.
        let job_id = job.id;

        persist_insert(&self.db, &job).await?;
        let _ = events.send(ScanEvent::JobUpdated(job.clone()));

        let (cancel_tx, cancel_rx) = watch::channel(false);
        self.active_jobs
            .lock()
            .expect("active_jobs mutex envenenado")
            .insert(job_id, cancel_tx);

        let db = self.db.clone();
        let clamav = self.clamav.clone();
        let active_jobs = self.active_jobs.clone();
        // Lida de forma resiliente com uma lista de exclusões corrompida ou
        // ilegível: nunca falha a verificação por causa disso, apenas
        // segue sem exclusões (equivalente ao comportamento anterior a
        // essa funcionalidade existir).
        let exclusions = self.get_scan_exclusions().await.unwrap_or_default();

        tokio::spawn(async move {
            run_job(
                &mut job, targets, exclusions, clamav, &db, cancel_rx, &events,
            )
            .await;
            active_jobs
                .lock()
                .expect("active_jobs mutex envenenado")
                .remove(&job_id);
        });

        Ok(job_id)
    }
}

/// Executa a tarefa até sua conclusão (ou falha/cancelamento), atualizando
/// `job` e emitindo eventos a cada transição relevante.
async fn run_job(
    job: &mut ScanJob,
    targets: Vec<PathBuf>,
    exclusions: Vec<String>,
    clamav: Option<Arc<ScanEngineAdapter>>,
    db: &Arc<Database>,
    cancel_rx: watch::Receiver<bool>,
    events: &mpsc::UnboundedSender<ScanEvent>,
) {
    if !transition(job, ScanStatus::Preparing) {
        return;
    }
    emit_and_persist(job, db, events).await;

    let Some(clamav) = clamav else {
        job.error_message = Some(
            "O mecanismo ClamAV não está disponível (executável não encontrado). \
             Instale o ClamAV ou configure o caminho em Configurações."
                .to_string(),
        );
        if transition(job, ScanStatus::Failed) {
            emit_and_persist(job, db, events).await;
        }
        return;
    };

    if !transition(job, ScanStatus::Running) {
        return;
    }
    emit_and_persist(job, db, events).await;

    let engine_version = clamav.probe_health().engine_version;
    let mut buffer: Vec<ScanResult> = Vec::new();
    // Marca de tempo do último arquivo processado, usada para medir a
    // duração real de cada entrada (tempo decorrido entre uma linha de
    // saída do clamscan e a anterior). Não é uma medição isolada e exata
    // do tempo de exame de um único arquivo (inclui qualquer buffering de
    // E/S entre linhas), mas é um valor real observado, nunca inventado.
    let mut last_entry_at = std::time::Instant::now();

    let run_result = clamav
        .run_scan(&targets, &exclusions, cancel_rx, SCAN_TIMEOUT, |entry| {
            let scan_result_id = ScanResultId::new();
            let (verdict, detections) = classify_entry(
                entry,
                ENGINE_NAME,
                engine_version.as_deref(),
                scan_result_id,
            );

            let now = std::time::Instant::now();
            let duration_ms = now
                .duration_since(last_entry_at)
                .as_millis()
                .min(u128::from(u64::MAX)) as u64;
            last_entry_at = now;

            job.files_scanned += 1;
            job.current_file = Some(entry.path.clone());
            if matches!(verdict, Verdict::Malicious | Verdict::Suspicious) {
                job.threats_found += 1;
            }

            let result = ScanResult {
                id: scan_result_id,
                scan_job_id: job.id,
                file_path: entry.path.clone(),
                // Hashing de arquivo ainda não é calculado nesta versão;
                // ver limitações em docs/roadmap.
                file_hash: None,
                engines_used: vec![ENGINE_NAME.to_string()],
                verdict,
                detections,
                duration_ms,
                examined_at: time::OffsetDateTime::now_utc(),
                error_message: None,
            };

            let _ = events.send(ScanEvent::JobUpdated(job.clone()));
            let _ = events.send(ScanEvent::FileResult(result.clone()));
            buffer.push(result);
        })
        .await;

    match run_result {
        Ok(_) => {
            if transition(job, ScanStatus::Completed) {
                persist_results(db, &buffer).await;
                emit_and_persist(job, db, events).await;
            }
        }
        Err(corvus_clamav::ClamAvError::Cancelled) => {
            // Running -> CancelRequested -> Cancelled, respeitando a
            // máquina de estados mesmo quando o pedido chega e é atendido
            // quase imediatamente.
            transition(job, ScanStatus::CancelRequested);
            if transition(job, ScanStatus::Cancelled) {
                persist_results(db, &buffer).await;
                emit_and_persist(job, db, events).await;
            }
        }
        Err(err) => {
            job.error_message = Some(err.to_string());
            if transition(job, ScanStatus::Failed) {
                persist_results(db, &buffer).await;
                emit_and_persist(job, db, events).await;
            }
        }
    }
}

fn transition(job: &mut ScanJob, next: ScanStatus) -> bool {
    match job.transition_to(next) {
        Ok(()) => true,
        Err(error) => {
            tracing::error!(%error, job_id = %job.id, "transição de estado de verificação rejeitada");
            false
        }
    }
}

async fn emit_and_persist(
    job: &ScanJob,
    db: &Arc<Database>,
    events: &mpsc::UnboundedSender<ScanEvent>,
) {
    // A persistência acontece ANTES da emissão do evento: o frontend reage a
    // eventos terminais recarregando o histórico direto do banco (ver
    // useScanEventBridge.ts), e se o evento saísse primeiro essa releitura
    // poderia correr contra a escrita ainda em andamento, lendo um estado
    // antigo (ex.: files_scanned desatualizado). Persistir primeiro garante
    // que, quando o evento chega, o estado já está durável.
    if let Err(error) = persist_update(db, job).await {
        tracing::error!(%error, job_id = %job.id, "falha ao persistir estado da verificação");
    }
    let _ = events.send(ScanEvent::JobUpdated(job.clone()));
}

async fn persist_insert(db: &Arc<Database>, job: &ScanJob) -> Result<(), ScannerError> {
    let db = db.clone();
    let job = job.clone();
    tokio::task::spawn_blocking(move || db.insert_scan_job(&job)).await??;
    Ok(())
}

async fn persist_update(db: &Arc<Database>, job: &ScanJob) -> Result<(), ScannerError> {
    let db = db.clone();
    let job = job.clone();
    tokio::task::spawn_blocking(move || db.update_scan_job(&job)).await??;
    Ok(())
}

async fn persist_results(db: &Arc<Database>, results: &[ScanResult]) {
    if results.is_empty() {
        return;
    }
    let db = db.clone();
    let results = results.to_vec();
    let outcome =
        tokio::task::spawn_blocking(move || -> Result<(), corvus_storage::StorageError> {
            for result in &results {
                db.insert_scan_result(result)?;
            }
            Ok(())
        })
        .await;

    match outcome {
        Ok(Ok(())) => {}
        Ok(Err(error)) => tracing::error!(%error, "falha ao persistir resultados de verificação"),
        Err(join_error) => tracing::error!(error = %join_error, "tarefa de persistência falhou"),
    }
}
