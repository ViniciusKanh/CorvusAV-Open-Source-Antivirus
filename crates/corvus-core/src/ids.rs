//! Identificadores fortemente tipados usados em todo o domínio do CorvusAV.
//!
//! Cada identificador envolve um `Uuid` v4 para evitar a troca acidental
//! entre tipos de ID distintos (por exemplo, um `ScanJobId` não deve poder
//! ser usado onde se espera um `DetectionId`).

use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;

macro_rules! typed_id {
    ($name:ident, $doc:expr) => {
        #[doc = $doc]
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $name(Uuid);

        impl $name {
            /// Gera um novo identificador aleatório (UUID v4).
            pub fn new() -> Self {
                Self(Uuid::new_v4())
            }

            /// Reconstrói o identificador a partir de um UUID já existente,
            /// por exemplo ao reidratar um registro vindo do SQLite.
            pub fn from_uuid(id: Uuid) -> Self {
                Self(id)
            }

            pub fn as_uuid(&self) -> Uuid {
                self.0
            }
        }

        impl Default for $name {
            fn default() -> Self {
                Self::new()
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "{}", self.0)
            }
        }
    };
}

typed_id!(ScanJobId, "Identificador de uma tarefa de verificação.");
typed_id!(ScanResultId, "Identificador do resultado de um arquivo examinado.");
typed_id!(DetectionId, "Identificador de uma detecção reportada por um motor.");
typed_id!(QuarantineItemId, "Identificador de um item em quarentena.");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_are_unique_and_roundtrip_through_serde() {
        let a = ScanJobId::new();
        let b = ScanJobId::new();
        assert_ne!(a, b);

        let json = serde_json::to_string(&a).unwrap();
        let back: ScanJobId = serde_json::from_str(&json).unwrap();
        assert_eq!(a, back);
    }
}
