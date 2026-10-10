use thiserror::Error;

#[derive(Debug, Error)]
pub enum PolicyError {
    #[error("nenhum caminho foi informado para a verificação personalizada")]
    EmptyTargetList,

    #[error("limite de {max} itens por verificação personalizada foi excedido ({got} informados)")]
    TooManyTargets { max: usize, got: usize },

    #[error("caminho não encontrado ou inacessível: {0}")]
    PathNotAccessible(String),
}
