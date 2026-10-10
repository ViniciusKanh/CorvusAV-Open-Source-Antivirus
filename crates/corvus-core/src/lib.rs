//! `corvus-core`: tipos de domínio, estados e erros compartilhados pelo
//! CorvusAV. Este crate não depende de infraestrutura (SQLite, processos
//! externos, IPC) — apenas modela os contratos que as demais camadas
//! implementam e consomem.

pub mod engine;
pub mod error;
pub mod ids;
pub mod quarantine;
pub mod result;
pub mod scan;

pub use engine::{EngineAvailability, EngineHealth};
pub use error::CoreError;
pub use ids::{DetectionId, QuarantineItemId, ScanJobId, ScanResultId};
pub use quarantine::{QuarantineItem, QuarantineState};
pub use result::{Detection, ScanResult, Verdict, VerdictCounts};
pub use scan::{ScanJob, ScanKind, ScanStatus};
