//! Isolamento básico de arquivos maliciosos/suspeitos do CorvusAV.
//!
//! Move o arquivo original para uma pasta protegida fora do caminho
//! original (nunca executa nem abre o arquivo), registra hash antes/depois
//! e todo o ciclo de vida em `quarantine_items`, e permite restauração
//! manual e auditável. **Não há quarentena automática nem exclusão
//! definitiva nesta versão** — apenas isolamento e restauração sob ação
//! explícita do usuário (ver corvusav-security-engineer skill, seção 5.3).

mod error;
mod hashing;
mod paths;

pub use error::QuarantineError;
pub use paths::resolve_quarantine_dir;

use corvus_core::{QuarantineItem, QuarantineItemId, QuarantineState, ScanResultId};
use corvus_storage::Database;
use hashing::sha256_file;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use time::OffsetDateTime;

pub struct QuarantineManager {
    db: Arc<Database>,
    quarantine_dir: PathBuf,
}

impl QuarantineManager {
    /// Garante que o diretório de quarentena existe e está com ACL
    /// restrita antes de aceitar qualquer operação.
    pub fn new(db: Arc<Database>) -> Result<Self, QuarantineError> {
        let quarantine_dir = resolve_quarantine_dir()?;
        std::fs::create_dir_all(&quarantine_dir)?;
        harden_directory_acl(&quarantine_dir);
        Ok(Self { db, quarantine_dir })
    }

    pub fn quarantine_dir(&self) -> &Path {
        &self.quarantine_dir
    }

    /// Move `original_path` para a quarentena e registra o item.
    ///
    /// `reason` deve descrever o motivo real da decisão (por exemplo, o
    /// veredito e a detecção que levaram o usuário a isolar o arquivo) —
    /// nunca um texto genérico. `scan_result_id` associa o item ao
    /// resultado de origem quando houver um (pode ser `None` para um
    /// isolamento manual fora do fluxo de verificação).
    pub fn quarantine_file(
        &self,
        original_path: &Path,
        reason: String,
        scan_result_id: Option<ScanResultId>,
    ) -> Result<QuarantineItem, QuarantineError> {
        if !original_path.is_file() {
            return Err(QuarantineError::SourceNotFound(
                original_path.display().to_string(),
            ));
        }

        let hash_before = sha256_file(original_path)?;

        let item_id = QuarantineItemId::new();
        // Nome interno aleatório (o próprio UUID do item) e sem a extensão
        // original — nunca o nome/extensão do arquivo real, para reduzir o
        // risco de execução acidental por associação de tipo de arquivo.
        let internal_name = format!("{}.quarantine", item_id.as_uuid());
        let destination = self.quarantine_dir.join(&internal_name);

        move_file(original_path, &destination)?;

        let hash_after = match sha256_file(&destination) {
            Ok(hash) => Some(hash),
            Err(e) => {
                // O arquivo já está fisicamente isolado (o que importa do
                // ponto de vista de segurança) mesmo que não consigamos
                // reler o hash agora; registramos a falha real em vez de
                // inventar um hash ou fingir que a verificação passou.
                tracing::warn!("falha ao recalcular hash após isolar {}: {e}", destination.display());
                None
            }
        };

        if let Some(after) = hash_after.as_deref() {
            if after != hash_before {
                tracing::warn!(
                    "hash mudou durante o isolamento de {}: {} -> {}",
                    original_path.display(),
                    hash_before,
                    after
                );
            }
        }

        let item = QuarantineItem {
            id: item_id,
            scan_result_id,
            original_path: original_path.display().to_string(),
            internal_name,
            state: QuarantineState::Quarantined,
            reason,
            file_hash_before: Some(hash_before),
            file_hash_after: hash_after,
            quarantined_at: OffsetDateTime::now_utc(),
            restored_at: None,
            last_error: None,
        };

        self.db.insert_quarantine_item(&item)?;
        Ok(item)
    }

    pub fn list_items(&self) -> Result<Vec<QuarantineItem>, QuarantineError> {
        Ok(self.db.list_quarantine_items()?)
    }

    /// Restaura um item para seu caminho original.
    ///
    /// Recusa a restauração (sem mover nada) se o destino original já
    /// existir, ou se o hash do arquivo isolado não bater mais com o
    /// registrado em `file_hash_after` (possível adulteração enquanto
    /// estava em quarentena). Nunca executa nem abre o arquivo restaurado.
    pub fn restore_item(&self, id: QuarantineItemId) -> Result<QuarantineItem, QuarantineError> {
        let mut item = self
            .db
            .get_quarantine_item(id)?
            .ok_or_else(|| QuarantineError::NotFound(id.to_string()))?;

        if !matches!(
            item.state,
            QuarantineState::Quarantined | QuarantineState::RestoreFailed
        ) {
            return Err(QuarantineError::InvalidState(format!("{:?}", item.state)));
        }

        let quarantined_path = self.quarantine_dir.join(&item.internal_name);
        if !quarantined_path.is_file() {
            self.fail_restore(&mut item, "O arquivo isolado não foi encontrado em disco (pode ter sido removido manualmente fora do CorvusAV).".to_string())?;
            return Err(QuarantineError::QuarantinedFileMissing);
        }

        let original = PathBuf::from(&item.original_path);
        if original.exists() {
            let message = format!(
                "Já existe um arquivo em \"{}\". Mova-o ou remova-o antes de restaurar este item.",
                original.display()
            );
            self.fail_restore(&mut item, message.clone())?;
            return Err(QuarantineError::DestinationExists(message));
        }

        if let Some(expected) = item.file_hash_after.clone() {
            match sha256_file(&quarantined_path) {
                Ok(actual) if actual == expected => {}
                Ok(_) => {
                    let message = "O arquivo em quarentena não corresponde mais ao hash registrado no isolamento; restauração recusada por segurança.".to_string();
                    self.fail_restore(&mut item, message.clone())?;
                    return Err(QuarantineError::IntegrityMismatch(message));
                }
                Err(e) => {
                    let message = format!("Não foi possível reler o hash do arquivo isolado: {e}");
                    self.fail_restore(&mut item, message.clone())?;
                    return Err(QuarantineError::Io(e));
                }
            }
        }

        if let Some(parent) = original.parent() {
            std::fs::create_dir_all(parent)?;
        }

        if let Err(e) = move_file(&quarantined_path, &original) {
            self.fail_restore(&mut item, e.to_string())?;
            return Err(e);
        }

        item.state = QuarantineState::Restored;
        item.restored_at = Some(OffsetDateTime::now_utc());
        item.last_error = None;
        self.db.update_quarantine_item(&item)?;
        Ok(item)
    }

    fn fail_restore(&self, item: &mut QuarantineItem, message: String) -> Result<(), QuarantineError> {
        item.state = QuarantineState::RestoreFailed;
        item.last_error = Some(message);
        self.db.update_quarantine_item(item)?;
        Ok(())
    }
}

/// Move um arquivo preservando atomicidade quando possível. Em caso de
/// origem e destino estarem em volumes diferentes (rename atômico
/// impossível), cai para copiar e só apagar o original depois de confirmar
/// que a cópia foi concluída com sucesso — nunca apaga antes de confirmar.
fn move_file(source: &Path, destination: &Path) -> Result<(), QuarantineError> {
    match std::fs::rename(source, destination) {
        Ok(()) => Ok(()),
        Err(e) if is_cross_volume_error(&e) => {
            std::fs::copy(source, destination)?;
            std::fs::remove_file(source)?;
            Ok(())
        }
        Err(e) => Err(QuarantineError::Io(e)),
    }
}

/// `ERROR_NOT_SAME_DEVICE` no Windows (e `EXDEV` no Unix, mesmo valor
/// numérico, 17) — a única condição em que caímos para copiar em vez de
/// renomear. Qualquer outro erro (permissão negada, arquivo em uso, etc.)
/// é propagado como está: nunca escondido atrás de uma tentativa de cópia
/// que provavelmente falharia pelo mesmo motivo real.
fn is_cross_volume_error(error: &std::io::Error) -> bool {
    error.raw_os_error() == Some(17)
}

/// Restringe a ACL da pasta de quarentena ao usuário atual (e contas de
/// sistema), via `icacls` — sem FFI insegura com a API do Windows. Melhor
/// esforço: a pasta já vive em `%LocalAppData%` do usuário atual, que o
/// próprio Windows não deixa outros usuários padrão lerem; isto é uma
/// camada adicional de defesa em profundidade, não a única barreira, então
/// uma falha aqui é registrada mas não impede a quarentena de funcionar.
fn harden_directory_acl(dir: &Path) {
    let username = match std::env::var("USERNAME") {
        Ok(name) if !name.is_empty() => name,
        _ => {
            tracing::warn!("não foi possível determinar o usuário atual para restringir a ACL da pasta de quarentena");
            return;
        }
    };

    let result = std::process::Command::new("icacls")
        .arg(dir)
        .arg("/inheritance:r")
        .arg("/grant:r")
        .arg(format!("{username}:(OI)(CI)F"))
        .arg("/grant:r")
        .arg("SYSTEM:(OI)(CI)F")
        .arg("/grant:r")
        .arg("Administrators:(OI)(CI)F")
        .output();

    match result {
        Ok(output) if output.status.success() => {}
        Ok(output) => {
            let stderr = String::from_utf8_lossy(&output.stderr);
            tracing::warn!("icacls não conseguiu restringir a pasta de quarentena: {stderr}");
        }
        Err(e) => {
            tracing::warn!("não foi possível executar icacls na pasta de quarentena: {e}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn manager_with_dir(dir: &Path) -> QuarantineManager {
        std::fs::create_dir_all(dir).unwrap();
        QuarantineManager {
            db: Arc::new(Database::open_in_memory().unwrap()),
            quarantine_dir: dir.to_path_buf(),
        }
    }

    #[test]
    fn quarantine_then_restore_roundtrips_file_back_to_original_path() {
        let base = std::env::temp_dir().join(format!("corvusav-test-quarantine-{}", std::process::id()));
        let source_dir = base.join("origem");
        let quarantine_dir = base.join("quarentena");
        std::fs::create_dir_all(&source_dir).unwrap();
        let original = source_dir.join("eicar.com");
        std::fs::write(&original, b"conteudo de teste, nao e malware real").unwrap();

        let manager = manager_with_dir(&quarantine_dir);

        let item = manager
            .quarantine_file(&original, "teste: ameaça simulada".to_string(), None)
            .expect("isolar deveria funcionar");
        assert!(!original.exists(), "arquivo original deveria ter sido movido");
        assert_eq!(item.file_hash_before, item.file_hash_after);

        let restored = manager.restore_item(item.id).expect("restaurar deveria funcionar");
        assert!(original.is_file(), "arquivo deveria voltar ao caminho original");
        assert_eq!(restored.state, QuarantineState::Restored);

        std::fs::remove_dir_all(&base).ok();
    }

    #[test]
    fn restoring_over_an_existing_file_is_refused_without_overwriting() {
        let base = std::env::temp_dir().join(format!("corvusav-test-quarantine-conflict-{}", std::process::id()));
        let source_dir = base.join("origem");
        let quarantine_dir = base.join("quarentena");
        std::fs::create_dir_all(&source_dir).unwrap();
        let original = source_dir.join("eicar.com");
        std::fs::write(&original, b"conteudo original").unwrap();

        let manager = manager_with_dir(&quarantine_dir);
        let item = manager
            .quarantine_file(&original, "teste".to_string(), None)
            .unwrap();

        // Um novo arquivo aparece no caminho original antes da restauração
        // (ex.: o usuário recriou o arquivo manualmente).
        std::fs::write(&original, b"outro conteudo, nao deve ser sobrescrito").unwrap();

        let err = manager.restore_item(item.id).unwrap_err();
        assert!(matches!(err, QuarantineError::DestinationExists(_)));

        let content = std::fs::read(&original).unwrap();
        assert_eq!(content, b"outro conteudo, nao deve ser sobrescrito");

        std::fs::remove_dir_all(&base).ok();
    }

    #[test]
    fn quarantining_a_missing_file_fails_instead_of_fabricating_an_item() {
        let base = std::env::temp_dir().join(format!("corvusav-test-quarantine-missing-{}", std::process::id()));
        let quarantine_dir = base.join("quarentena");
        let manager = manager_with_dir(&quarantine_dir);

        let missing = base.join("nao-existe.bin");
        let err = manager
            .quarantine_file(&missing, "teste".to_string(), None)
            .unwrap_err();
        assert!(matches!(err, QuarantineError::SourceNotFound(_)));

        std::fs::remove_dir_all(&base).ok();
    }
}
