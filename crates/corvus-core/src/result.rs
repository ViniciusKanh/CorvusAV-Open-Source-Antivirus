//! Resultados de verificação, vereditos e detecções.

use crate::ids::{DetectionId, ScanJobId, ScanResultId};
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

/// Veredito final atribuído a um arquivo examinado.
///
/// `Error` e `Unknown` são estados de primeira classe, distintos de `Clean`:
/// um mecanismo que falhou ou retornou um resultado indeterminado **nunca**
/// deve ser tratado como "sem ameaças". Ver [`Verdict::combine`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Verdict {
    Clean,
    Suspicious,
    Malicious,
    Unknown,
    Error,
}

impl Verdict {
    /// Ordem de severidade/cautela usada para agregar vereditos de múltiplos
    /// mecanismos sem nunca "rebaixar" um resultado mais cauteloso.
    ///
    /// Do mais severo ao menos cauteloso: `Malicious` > `Suspicious` >
    /// `Error` > `Unknown` > `Clean`. `Clean` só prevalece quando nenhum
    /// mecanismo relevante relatou outra coisa.
    fn severity_rank(self) -> u8 {
        match self {
            Verdict::Malicious => 4,
            Verdict::Suspicious => 3,
            Verdict::Error => 2,
            Verdict::Unknown => 1,
            Verdict::Clean => 0,
        }
    }

    /// Combina dois vereditos (por exemplo, de mecanismos diferentes sobre o
    /// mesmo arquivo) preservando sempre o mais cauteloso. Nunca produz
    /// `Clean` a partir de uma combinação que contenha `Error` ou `Unknown`.
    pub fn combine(self, other: Verdict) -> Verdict {
        if self.severity_rank() >= other.severity_rank() {
            self
        } else {
            other
        }
    }

    /// Combina uma sequência de vereditos. Uma lista vazia é tratada como
    /// `Unknown` (nenhum mecanismo examinou o arquivo), nunca como `Clean`.
    pub fn combine_all(verdicts: impl IntoIterator<Item = Verdict>) -> Verdict {
        verdicts
            .into_iter()
            .fold(None, |acc, v| {
                Some(match acc {
                    Some(current) => Verdict::combine(current, v),
                    None => v,
                })
            })
            .unwrap_or(Verdict::Unknown)
    }
}

/// Contagem de vereditos de uma verificação, usada para dar visibilidade
/// rápida da "qualidade" de uma varredura (quantos itens vieram limpos,
/// suspeitos, maliciosos, indeterminados ou com erro) sem precisar expandir
/// cada resultado individual no histórico. Deliberadamente não compõe os
/// vereditos em uma única pontuação — ver o aviso em
/// [`Verdict::combine`] sobre nunca misturar vereditos heterogêneos em um
/// só número.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct VerdictCounts {
    pub clean: u64,
    pub suspicious: u64,
    pub malicious: u64,
    pub unknown: u64,
    pub error: u64,
}

impl VerdictCounts {
    pub fn add(&mut self, verdict: Verdict) {
        match verdict {
            Verdict::Clean => self.clean += 1,
            Verdict::Suspicious => self.suspicious += 1,
            Verdict::Malicious => self.malicious += 1,
            Verdict::Unknown => self.unknown += 1,
            Verdict::Error => self.error += 1,
        }
    }

    pub fn total(&self) -> u64 {
        self.clean + self.suspicious + self.malicious + self.unknown + self.error
    }
}

/// Uma detecção específica reportada por um mecanismo para um resultado.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Detection {
    pub id: DetectionId,
    pub scan_result_id: ScanResultId,
    pub engine_name: String,
    pub engine_version: Option<String>,
    /// Identificador da assinatura/regra que motivou a detecção.
    pub signature_id: String,
    /// Metadados adicionais específicos do mecanismo (não deve conter
    /// conteúdo do arquivo examinado).
    pub metadata: serde_json::Value,
}

/// Resultado da verificação de um único arquivo.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanResult {
    pub id: ScanResultId,
    pub scan_job_id: ScanJobId,
    pub file_path: String,
    pub file_hash: Option<String>,
    pub engines_used: Vec<String>,
    pub verdict: Verdict,
    pub detections: Vec<Detection>,
    pub duration_ms: u64,
    pub examined_at: OffsetDateTime,
    /// Presente quando o veredito é `Error`: descreve a falha do mecanismo.
    pub error_message: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn combine_never_downgrades_to_clean() {
        assert_eq!(
            Verdict::combine(Verdict::Clean, Verdict::Error),
            Verdict::Error
        );
        assert_eq!(
            Verdict::combine(Verdict::Clean, Verdict::Unknown),
            Verdict::Unknown
        );
        assert_eq!(
            Verdict::combine(Verdict::Suspicious, Verdict::Malicious),
            Verdict::Malicious
        );
    }

    #[test]
    fn combine_all_of_empty_is_unknown_not_clean() {
        assert_eq!(Verdict::combine_all(std::iter::empty()), Verdict::Unknown);
    }

    #[test]
    fn combine_is_symmetric() {
        for a in [
            Verdict::Clean,
            Verdict::Suspicious,
            Verdict::Malicious,
            Verdict::Unknown,
            Verdict::Error,
        ] {
            for b in [
                Verdict::Clean,
                Verdict::Suspicious,
                Verdict::Malicious,
                Verdict::Unknown,
                Verdict::Error,
            ] {
                assert_eq!(Verdict::combine(a, b), Verdict::combine(b, a));
            }
        }
    }
}
