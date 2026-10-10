//! Execução do `freshclam` para atualizar manualmente a base de assinaturas
//! do ClamAV. Processo externo isolado (sem shell), disparado só quando o
//! usuário pede explicitamente — nunca em segundo plano, nunca agendado
//! automaticamente nesta versão.

use std::path::Path;
use std::process::Stdio;

use serde::Serialize;
use thiserror::Error;
use tokio::process::Command;

use crate::detect::locate_freshclam;

#[derive(Debug, Error)]
pub enum FreshclamError {
    #[error(
        "freshclam não foi encontrado. Instale o ClamAV e/ou configure a variável de \
         ambiente CORVUSAV_CLAMAV_PATH apontando para o diretório do clamscan."
    )]
    NotFound,
    #[error("falha de E/S ao executar o freshclam: {0}")]
    Io(#[from] std::io::Error),
}

/// Resultado real (nunca simulado) de uma tentativa de atualização de
/// assinaturas. `success` reflete só o código de saída do `freshclam`;
/// `output` traz stdout+stderr concatenados exatamente como o processo os
/// produziu, para que a interface possa mostrar ao usuário o motivo real de
/// uma falha (por exemplo, permissão de escrita negada em "Program Files")
/// em vez de uma mensagem genérica.
#[derive(Debug, Clone, Serialize)]
pub struct FreshclamOutcome {
    pub success: bool,
    pub exit_code: Option<i32>,
    pub output: String,
}

/// Localiza e executa o `freshclam` sem privilégios elevados — exatamente
/// como o usuário faria rodando o executável diretamente. Em instalações do
/// ClamAV dentro de `Program Files`, isso tende a falhar por falta de
/// permissão de escrita no diretório de assinaturas; este MVP não tenta
/// elevar privilégios automaticamente (ver docs/roadmap), então, nesse
/// caso, o erro real relatado pelo próprio `freshclam` é repassado à
/// interface em vez de ser escondido ou interpretado como sucesso.
pub async fn run_freshclam() -> Result<FreshclamOutcome, FreshclamError> {
    let executable = locate_freshclam().ok_or(FreshclamError::NotFound)?;
    run_freshclam_at(&executable).await
}

async fn run_freshclam_at(executable: &Path) -> Result<FreshclamOutcome, FreshclamError> {
    let output = Command::new(executable)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await?;

    let mut combined = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr);
    if !stderr.trim().is_empty() {
        if !combined.is_empty() {
            combined.push('\n');
        }
        combined.push_str(stderr.trim());
    }

    Ok(FreshclamOutcome {
        success: output.status.success(),
        exit_code: output.status.code(),
        output: combined,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn nonexistent_executable_never_reports_fake_success() {
        // Um caminho de executável inválido deve propagar o erro real de
        // E/S do sistema operacional — nunca ser interpretado como uma
        // atualização bem-sucedida.
        let result = run_freshclam_at(Path::new("freshclam-que-nao-existe.exe")).await;
        assert!(result.is_err());
    }
}
