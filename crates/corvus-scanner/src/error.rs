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

    #[error("falha ao (des)serializar configuração: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error(transparent)]
    Freshclam(#[from] corvus_clamav::FreshclamError),

    #[error(transparent)]
    Quarantine(#[from] corvus_quarantine::QuarantineError),

    #[error("resultado de verificação não encontrado: {0}")]
    ScanResultNotFound(corvus_core::ScanResultId),

    #[error(
        "o recurso de quarentena não está disponível nesta sessão (falha ao preparar o diretório de armazenamento — veja os logs para o motivo real)"
    )]
    QuarantineUnavailable,
}
