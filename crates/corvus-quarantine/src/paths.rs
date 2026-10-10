//! Resolução do diretório de armazenamento da quarentena.

use crate::error::QuarantineError;
use std::path::PathBuf;

const APP_DIR_NAME: &str = "CorvusAV";
const QUARANTINE_DIR_NAME: &str = "quarantine";

/// Diretório onde os arquivos isolados ficam fisicamente armazenados.
///
/// Fica em `%LocalAppData%\CorvusAV\quarantine` nesta versão — o mesmo
/// nível de privilégio do banco de dados (ver
/// `corvus_storage::resolve_data_dir`), porque o CorvusAV ainda não tem um
/// serviço Windows privilegiado rodando com uma conta própria (ver
/// docs/architecture). O plano original falava em `%ProgramData%`, mas essa
/// pasta pressupõe o serviço privilegiado que ainda não existe nesta
/// versão; usar `%ProgramData%` sem ele exigiria elevação que o app não
/// tem, ou deixaria a pasta sem o isolamento de ACL que o serviço daria.
/// Migrar para `%ProgramData%` com ACLs geridas pelo serviço fica como
/// pendência documentada para quando esse serviço existir.
pub fn resolve_quarantine_dir() -> Result<PathBuf, QuarantineError> {
    let base = dirs::data_local_dir().ok_or(QuarantineError::QuarantineDirUnavailable)?;
    Ok(base.join(APP_DIR_NAME).join(QUARANTINE_DIR_NAME))
}
