//! `ScannerService`: ponto único de orquestração de verificações, chamado
//! pelos comandos Tauri. Mantém o estado das tarefas ativas (para suportar
//! cancelamento) e delega classificação/política a `corvus-policy` e
//! execução de mecanismo a `corvus-clamav`.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex as StdMutex};
use std::time::Duration;

use corvus_clamav::ClamAvAdapter;
use corvus_core::{EngineHealth, ScanJob, ScanJobId, ScanKind, ScanResult, ScanResultId, ScanStatus, Verdict};
use corvus_policy::{classify_entry, full_scan_targets, quick_scan_targets, validate_custom_targets, FullScanAvailability};
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
    clamav: Option<Arc<ClamAvAdapter>>,
    active_jobs: Arc<StdMutex<HashMap<ScanJobId, watch::Sender<bool>>>>,
}

impl ScannerService {
    pub fn new(db: Arc<Database>, clamav: Option<ClamAvAdapter>) -> Self {
        Self {
            db,
            clamav: clamav.map(Arc::new),
            active_jobs: Arc::new(StdMutex::new(HashMap::new())),
        }
    }

    /// Saúde atual do mecanismo de varredura (nunca reporta `Available` sem
    /// ter sondado o executável real).
    pub async fn engine_health(&self) -> EngineHealth {
        let executable = self.clamav.as_ref().map(|a| a.executable_path().to_path_buf());
        let result = tokio::task::spawn_blocking(move || {
            corvus_clamav::probe_health(executable.as_deref())
        })
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

    /// Solicita o cancelamento cooperativo de uma tarefa em execução.
    /// Retorna erro se a tarefa não estiver mais ativa (já concluída,
    /// cancelada ou com falha).
    pub fn cancel_scan(&self, job_id: ScanJobId) -> Result<(), ScannerError> {
        let jobs = self.active_jobs.lock().expect("active_jobs mutex envenenado");
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

        let target_strings: Vec<String> =
            targets.iter().map(|p| p.display().to_string()).collect();
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

        tokio::spawn(async move {
            run_job(&mut job, targets, clamav, &db, cancel_rx, &events).await;
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
    clamav: Option<Arc<ClamAvAdapter>>,
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

    let run_result = clamav
        .run_scan(&targets, cancel_rx, SCAN_TIMEOUT, |entry| {
            let scan_result_id = ScanResultId::new();
            let (verdict, detections) =
                classify_entry(entry, ENGINE_NAME, engine_version.as_deref(), scan_result_id);

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
                duration_ms: 0,
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
    let _ = events.send(ScanEvent::JobUpdated(job.clone()));
    if let Err(error) = persist_update(db, job).await {
        tracing::error!(%error, job_id = %job.id, "falha ao persistir estado da verificação");
    }
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
    let outcome = tokio::task::spawn_blocking(move || -> Result<(), corvus_storage::StorageError> {
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
