//! `corvus-storage`: persistência SQLite do CorvusAV — migrations,
//! histórico de verificações, estado dos mecanismos e configurações.
//!
//! O banco vive no diretório de dados do usuário (ver [`paths`]), nunca
//! dentro dos recursos somente leitura do executável instalado.

mod codec;
mod migrations;

pub mod database;
pub mod error;
pub mod paths;

pub use database::{AuditEvent, Database};
pub use error::{StorageError, StorageResult};
pub use paths::{resolve_data_dir, resolve_db_path};
