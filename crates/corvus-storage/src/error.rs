use thiserror::Error;

#[derive(Debug, Error)]
pub enum StorageError {
    #[error("erro de banco de dados: {0}")]
    Database(#[from] rusqlite::Error),

    #[error("erro de serialização: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("erro de E/S: {0}")]
    Io(#[from] std::io::Error),

    #[error("não foi possível determinar o diretório de dados do CorvusAV")]
    DataDirUnavailable,

    #[error("registro não encontrado: {0}")]
    NotFound(String),

    #[error("erro ao converter data/hora: {0}")]
    InvalidTimestamp(String),
}

pub type StorageResult<T> = Result<T, StorageError>;
