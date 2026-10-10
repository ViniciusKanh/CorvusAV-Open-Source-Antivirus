//! Estados e contratos de uma tarefa de verificação (`ScanJob`).

use crate::error::CoreError;
use crate::ids::ScanJobId;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

/// Tipo de verificação solicitada pelo usuário.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScanKind {
    /// Examina um conjunto explícito e documentado de diretórios críticos.
    Quick,
    /// Examina as unidades/diretórios definidos pela política vigente.
    Full,
    /// Examina arquivos ou diretórios escolhidos explicitamente pelo usuário.
    Custom,
}

/// Estado do ciclo de vida de uma `ScanJob`.
///
/// As transições válidas são impostas por [`ScanJob::transition_to`]; nenhuma
/// camada deve escrever o status diretamente sem passar por essa validação.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScanStatus {
    Queued,
    Preparing,
    Running,
    CancelRequested,
    Completed,
    Failed,
    Cancelled,
}

impl ScanStatus {
    /// Indica se o status representa uma tarefa já finalizada (não pode
    /// mais transicionar para nenhum outro estado).
    pub fn is_terminal(self) -> bool {
        matches!(self, Self::Completed | Self::Failed | Self::Cancelled)
    }

    fn can_transition_to(self, next: Self) -> bool {
        use ScanStatus::*;
        matches!(
            (self, next),
            (Queued, Preparing)
                | (Queued, Failed)
                | (Queued, Cancelled)
                | (Preparing, Running)
                | (Preparing, Failed)
                | (Preparing, Cancelled)
                | (Running, CancelRequested)
                | (Running, Completed)
                | (Running, Failed)
                | (CancelRequested, Cancelled)
                | (CancelRequested, Completed)
                | (CancelRequested, Failed)
        )
    }
}

/// Uma tarefa de verificação, da fila até a conclusão.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanJob {
    pub id: ScanJobId,
    pub kind: ScanKind,
    pub status: ScanStatus,
    /// Caminhos-alvo explícitos (obrigatório para `ScanKind::Custom`,
    /// documentado/derivado da política para `Quick`/`Full`).
    pub target_paths: Vec<String>,
    pub created_at: OffsetDateTime,
    pub started_at: Option<OffsetDateTime>,
    pub finished_at: Option<OffsetDateTime>,
    /// Caminho (resumido) do arquivo sendo examinado no momento, quando
    /// permitido pela política de exibição.
    pub current_file: Option<String>,
    pub files_scanned: u64,
    /// Total conhecido de arquivos a examinar, quando disponível. Sem total
    /// conhecido, a UI não deve apresentar progresso percentual.
    pub files_total: Option<u64>,
    pub threats_found: u64,
    pub error_message: Option<String>,
}

impl ScanJob {
    pub fn new(kind: ScanKind, target_paths: Vec<String>) -> Self {
        Self {
            id: ScanJobId::new(),
            kind,
            status: ScanStatus::Queued,
            target_paths,
            created_at: OffsetDateTime::now_utc(),
            started_at: None,
            finished_at: None,
            current_file: None,
            files_scanned: 0,
            files_total: None,
            threats_found: 0,
            error_message: None,
        }
    }

    /// Aplica uma transição de estado, validando que ela é permitida.
    /// Retorna erro em vez de aceitar silenciosamente uma transição inválida.
    pub fn transition_to(&mut self, next: ScanStatus) -> Result<(), CoreError> {
        if !self.status.can_transition_to(next) {
            return Err(CoreError::InvalidStateTransition {
                from: format!("{:?}", self.status),
                to: format!("{:?}", next),
            });
        }

        if next == ScanStatus::Running && self.started_at.is_none() {
            self.started_at = Some(OffsetDateTime::now_utc());
        }
        if next.is_terminal() {
            self.finished_at = Some(OffsetDateTime::now_utc());
        }

        self.status = next;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_transition_sequence_succeeds() {
        let mut job = ScanJob::new(ScanKind::Quick, vec![]);
        assert_eq!(job.status, ScanStatus::Queued);

        job.transition_to(ScanStatus::Preparing).unwrap();
        job.transition_to(ScanStatus::Running).unwrap();
        assert!(job.started_at.is_some());

        job.transition_to(ScanStatus::Completed).unwrap();
        assert!(job.status.is_terminal());
        assert!(job.finished_at.is_some());
    }

    #[test]
    fn skipping_states_is_rejected() {
        let mut job = ScanJob::new(ScanKind::Custom, vec!["C:/Users/alguem/Downloads".into()]);
        let err = job.transition_to(ScanStatus::Completed).unwrap_err();
        assert!(matches!(err, CoreError::InvalidStateTransition { .. }));
    }

    #[test]
    fn terminal_states_do_not_accept_further_transitions() {
        let mut job = ScanJob::new(ScanKind::Quick, vec![]);
        job.transition_to(ScanStatus::Preparing).unwrap();
        job.transition_to(ScanStatus::Failed).unwrap();
        assert!(job.transition_to(ScanStatus::Running).is_err());
    }
}
