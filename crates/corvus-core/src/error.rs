use thiserror::Error;

/// Erros de domínio do CorvusAV, independentes de infraestrutura
/// (SQLite, processos externos, IPC). Camadas de infraestrutura devem
/// converter seus próprios erros para `CoreError` ou propagar o contexto
/// original através de seus próprios tipos de erro.
#[derive(Debug, Error)]
pub enum CoreError {
    #[error("transição de estado inválida: {from} -> {to}")]
    InvalidStateTransition { from: String, to: String },

    #[error("valor inválido: {0}")]
    InvalidValue(String),
}
