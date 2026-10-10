//! `corvus-scanner`: orquestração das tarefas de verificação do CorvusAV.

mod error;
mod events;
mod service;

pub use error::ScannerError;
pub use events::ScanEvent;
pub use service::ScannerService;
