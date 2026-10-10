//! `corvus-clamav`: adaptador do CorvusAV para o mecanismo ClamAV, executado
//! como processo externo isolado (sem shell, sem `libclamav` embarcado
//! nesta fase). Expõe `ScanEngineAdapter` como ponto único de entrada para
//! `corvus-scanner`: ele tenta o `clamd` (ver `clamd`/`engine`, mais rápido
//! em verificações grandes por varrer com múltiplas threads) quando
//! disponível, e cai automaticamente para o `clamscan` (ver `adapter`)
//! quando não — `ClamAvAdapter`/`ClamdAdapter` continuam expostos
//! individualmente para quem precisar de um dos dois especificamente.

mod adapter;
mod clamd;
mod detect;
mod engine;
mod filelist;
mod freshclam;

pub use adapter::{ClamAvAdapter, ClamAvError, RawScanEntry, RawVerdict, parse_clamscan_line};
pub use clamd::{ClamdAdapter, ClamdError};
pub use detect::{locate_clamd, locate_clamscan, locate_freshclam, probe_health, resolve_database_dir, EngineVersionInfo};
pub use engine::ScanEngineAdapter;
pub use freshclam::{run_freshclam, FreshclamError, FreshclamOutcome};
