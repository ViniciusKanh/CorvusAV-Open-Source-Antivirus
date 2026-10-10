use thiserror::Error;

#[derive(Debug, Error)]
pub enum QuarantineError {
    #[error("arquivo de origem não encontrado ou não é um arquivo regular: {0}")]
    SourceNotFound(String),

    #[error("erro de E/S ao isolar/restaurar arquivo: {0}")]
    Io(#[from] std::io::Error),

    #[error("erro de armazenamento: {0}")]
    Storage(#[from] corvus_storage::StorageError),

    #[error("não foi possível determinar o diretório de quarentena")]
    QuarantineDirUnavailable,

    #[error("item de quarentena não encontrado: {0}")]
    NotFound(String),

    #[error("operação inválida para o estado atual do item ({0})")]
    InvalidState(String),

    #[error("o arquivo isolado não foi encontrado em disco (pode ter sido removido manualmente fora do CorvusAV)")]
    QuarantinedFileMissing,

    #[error("{0}")]
    DestinationExists(String),

    #[error("{0}")]
    IntegrityMismatch(String),
}
