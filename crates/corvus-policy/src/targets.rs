//! Políticas de seleção de alvos para cada tipo de verificação.

use std::path::{Path, PathBuf};

use crate::error::PolicyError;

/// Limite de itens aceitos em uma verificação personalizada em um único
/// incremento. Evita que uma seleção acidental (ex.: a raiz de uma unidade)
/// vire, sem aviso, uma verificação completa não implementada.
pub const MAX_CUSTOM_TARGETS: usize = 64;

/// Resultado da política de verificação completa nesta versão.
pub enum FullScanAvailability {
    Available(Vec<PathBuf>),
    /// A verificação completa ainda não está implementada: a interface deve
    /// comunicar isso claramente, nunca simular uma varredura completa.
    Unavailable {
        reason: String,
    },
}

/// Conjunto explícito e documentado de diretórios críticos para a
/// verificação rápida: downloads, área de trabalho, documentos e o
/// diretório temporário do usuário atual. Não cobre o sistema por inteiro —
/// a interface deve comunicar esse escopo, nunca prometer cobertura total.
pub fn quick_scan_targets() -> Vec<PathBuf> {
    let mut targets = Vec::new();

    if let Some(dir) = dirs::download_dir() {
        targets.push(dir);
    }
    if let Some(dir) = dirs::desktop_dir() {
        targets.push(dir);
    }
    if let Some(dir) = dirs::document_dir() {
        targets.push(dir);
    }
    targets.push(std::env::temp_dir());

    targets.retain(|p| p.is_dir());
    targets.dedup();
    targets
}

/// A verificação completa (unidades/diretórios conforme política, exclusões
/// e limites) ainda não foi implementada nesta primeira versão. Retornar
/// essa condição explicitamente evita que a UI apresente uma opção que não
/// tem implementação funcional correspondente por trás.
pub fn full_scan_targets() -> FullScanAvailability {
    FullScanAvailability::Unavailable {
        reason: "A verificação completa (todas as unidades, com exclusões e limites \
                 configuráveis) será implementada em uma fase posterior do roadmap. \
                 Use a verificação rápida ou selecione pastas específicas na \
                 verificação personalizada."
            .to_string(),
    }
}

/// Valida os alvos escolhidos pelo usuário na verificação personalizada:
/// lista não vazia, dentro do limite, e cada caminho de fato acessível no
/// momento da validação (verificação adicional de condição de corrida ainda
/// cabe à camada de execução, que deve tratar arquivos que desapareçam ou
/// se tornem inacessíveis entre a validação e a varredura).
pub fn validate_custom_targets(paths: &[PathBuf]) -> Result<Vec<PathBuf>, PolicyError> {
    if paths.is_empty() {
        return Err(PolicyError::EmptyTargetList);
    }
    if paths.len() > MAX_CUSTOM_TARGETS {
        return Err(PolicyError::TooManyTargets {
            max: MAX_CUSTOM_TARGETS,
            got: paths.len(),
        });
    }

    for path in paths {
        if !path_is_accessible(path) {
            return Err(PolicyError::PathNotAccessible(path.display().to_string()));
        }
    }

    let mut validated = paths.to_vec();
    validated.dedup();
    Ok(validated)
}

fn path_is_accessible(path: &Path) -> bool {
    std::fs::symlink_metadata(path).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quick_scan_targets_only_includes_directories_that_exist() {
        for target in quick_scan_targets() {
            assert!(
                target.is_dir(),
                "{target:?} deveria existir e ser um diretório"
            );
        }
    }

    #[test]
    fn full_scan_is_explicitly_unavailable_in_v0_1_0() {
        match full_scan_targets() {
            FullScanAvailability::Unavailable { reason } => assert!(!reason.is_empty()),
            FullScanAvailability::Available(_) => {
                panic!("verificação completa não deveria estar disponível nesta versão")
            }
        }
    }

    #[test]
    fn empty_custom_target_list_is_rejected() {
        assert!(matches!(
            validate_custom_targets(&[]),
            Err(PolicyError::EmptyTargetList)
        ));
    }

    #[test]
    fn nonexistent_custom_target_is_rejected() {
        let bogus = PathBuf::from("/caminho/que/certamente/nao/existe/corvusav-teste");
        assert!(matches!(
            validate_custom_targets(&[bogus]),
            Err(PolicyError::PathNotAccessible(_))
        ));
    }

    #[test]
    fn too_many_targets_is_rejected() {
        let many: Vec<PathBuf> = (0..MAX_CUSTOM_TARGETS + 1)
            .map(|i| std::env::temp_dir().join(format!("corvusav-{i}")))
            .collect();
        // Mesmo que não existam, o limite de quantidade deve ser checado
        // antes da verificação de acessibilidade para dar um erro mais
        // claro ao usuário.
        let err = validate_custom_targets(&many).unwrap_err();
        assert!(matches!(err, PolicyError::TooManyTargets { .. }));
    }
}
