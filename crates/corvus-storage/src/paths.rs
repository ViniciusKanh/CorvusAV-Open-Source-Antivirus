//! Resolução dos diretórios de dados do CorvusAV.
//!
//! Na v0.1.0 o CorvusAV opera inteiramente no contexto do usuário, sem
//! serviço Windows privilegiado. Por isso os dados (banco SQLite, logs)
//! ficam em `%LocalAppData%\CorvusAV\data` no Windows (e equivalentes em
//! outros sistemas, úteis para desenvolvimento/testes). Quando o serviço
//! Windows privilegiado for introduzido (ver docs/roadmap), o estado
//! compartilhado migra para `%ProgramData%\CorvusAV` com ACLs apropriadas —
//! essa mudança está documentada como decisão pendente em
//! docs/architecture.

use crate::error::{StorageError, StorageResult};
use std::fs;
use std::path::PathBuf;

const APP_DIR_NAME: &str = "CorvusAV";

/// Diretório base de dados do usuário atual para o CorvusAV, criando-o se
/// necessário. Nunca aponta para dentro dos recursos somente leitura do
/// executável instalado.
pub fn resolve_data_dir() -> StorageResult<PathBuf> {
    let base = dirs::data_local_dir().ok_or(StorageError::DataDirUnavailable)?;
    let dir = base.join(APP_DIR_NAME).join("data");
    fs::create_dir_all(&dir)?;
    Ok(dir)
}

/// Caminho completo do arquivo de banco de dados SQLite principal.
pub fn resolve_db_path() -> StorageResult<PathBuf> {
    Ok(resolve_data_dir()?.join("corvusav.db"))
}
