use corvus_core::ScanJobId;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ScannerError {
    #[error("{0}")]
    ScanKindUnavailable(String),

    #[error("alvos personalizados não foram informados para a verificação")]
    MissingCustomTargets,

    #[error(transparent)]
    Policy(#[from] corvus_policy::PolicyError),

    #[error(transparent)]
    Storage(#[from] corvus_storage::StorageError),

    #[error("falha interna de tarefa assíncrona: {0}")]
    Join(#[from] tokio::task::JoinError),

    #[error("a tarefa {0} não está em execução ou já foi concluída")]
    JobNotActive(ScanJobId),
}
