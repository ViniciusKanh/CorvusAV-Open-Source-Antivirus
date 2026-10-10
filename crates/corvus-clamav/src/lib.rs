//! `corvus-clamav`: adaptador do CorvusAV para o mecanismo ClamAV,
//! executado via `clamscan` como processo externo isolado (sem shell, sem
//! `libclamav` embarcado nesta fase).

mod adapter;
mod detect;

pub use adapter::{ClamAvAdapter, ClamAvError, RawScanEntry, RawVerdict, parse_clamscan_line};
pub use detect::{locate_clamscan, probe_health, EngineVersionInfo};
