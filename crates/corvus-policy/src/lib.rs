//! `corvus-policy`: regras de decisão do CorvusAV — classificação de
//! resultados brutos de mecanismos em vereditos/detecções do domínio, e
//! políticas de seleção de alvos para cada tipo de verificação.

mod classification;
mod error;
mod targets;

pub use classification::classify_entry;
pub use error::PolicyError;
pub use targets::{
    full_scan_targets, quick_scan_targets, validate_custom_targets, FullScanAvailability,
    MAX_CUSTOM_TARGETS,
};
