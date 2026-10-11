//! Enumeração dos arquivos a examinar numa verificação, pulando arquivos
//! "só na nuvem" (OneDrive e qualquer outro provedor que use o mesmo
//! mecanismo de Arquivos sob Demanda do Windows) em vez de deixar o
//! mecanismo de verificação forçar o download completo deles só para poder
//! ler o conteúdo — o que pode travar uma verificação por minutos num único
//! arquivo grande que ainda não foi baixado para o disco local.
//!
//! Motivação real: uma verificação rápida sobre Desktop/Documentos/
//! Downloads sincronizados via OneDrive ficou mais de 5 minutos sem
//! terminar de examinar 30 arquivos — consistente com o Windows baixando
//! arquivos grandes da nuvem um a um antes de cada leitura, não com uma
//! lentidão real de verificação.

use std::path::{Path, PathBuf};

/// Um arquivo que não entrou na verificação porque está marcado pelo
/// Windows como "só na nuvem" — nunca tratado como limpo pelo chamador
/// (ver o veredito `RawVerdict::Error` sintetizado para cada um em
/// `crate::adapter::ClamAvAdapter::run_scan`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkippedCloudFile {
    pub path: PathBuf,
}

/// Bit `FILE_ATTRIBUTE_RECALL_ON_DATA_ACCESS` do Windows: marca um arquivo
/// cujo conteúdo ainda não está presente no disco local (só o
/// metadado/atalho) — usado pelo OneDrive e por outros provedores de
/// sincronização com "Arquivos sob Demanda". Consultar este atributo via
/// metadados nunca dispara o download; só abrir o arquivo para leitura
/// dispara.
#[cfg(windows)]
const FILE_ATTRIBUTE_RECALL_ON_DATA_ACCESS: u32 = 0x0040_0000;

#[cfg(windows)]
fn is_cloud_only_placeholder(metadata: &std::fs::Metadata) -> bool {
    use std::os::windows::fs::MetadataExt;
    metadata.file_attributes() & FILE_ATTRIBUTE_RECALL_ON_DATA_ACCESS != 0
}

/// Fora do Windows (só usado em testes desta base de código, que roda em
/// Linux) não existe esse mecanismo — nunca pula nada por esse motivo.
#[cfg(not(windows))]
fn is_cloud_only_placeholder(_metadata: &std::fs::Metadata) -> bool {
    false
}

fn is_excluded(path: &Path, exclude_dirs: &[PathBuf]) -> bool {
    exclude_dirs
        .iter()
        .any(|excluded| path.starts_with(excluded))
}

/// Percorre recursivamente os diretórios em `targets`, retornando os
/// arquivos reais (presentes localmente) a examinar e, separadamente, os
/// que foram pulados por estarem só na nuvem. `exclude_dirs` usa o mesmo
/// formato já armazenado pela tela de Configurações (caminhos de diretório
/// completos); um arquivo ou subdiretório cujo caminho comece por uma
/// dessas entradas é inteiramente ignorado — mesma correspondência
/// literal (sensível a maiúsculas/minúsculas) já documentada em
/// `ScannerService::set_scan_exclusions`, agora aplicada aqui em vez de
/// via `--exclude-dir` do `clamscan`.
///
/// Só faz chamadas de metadado, nunca abre/lê o conteúdo de um arquivo —
/// então nunca força o download de um arquivo só na nuvem, mesmo para os
/// que acaba pulando.
pub fn enumerate_files(
    targets: &[PathBuf],
    exclude_dirs: &[String],
) -> (Vec<PathBuf>, Vec<SkippedCloudFile>) {
    let mut files = Vec::new();
    let mut skipped = Vec::new();
    let exclude_dirs: Vec<PathBuf> = exclude_dirs.iter().map(PathBuf::from).collect();

    for target in targets {
        if is_excluded(target, &exclude_dirs) {
            continue;
        }

        // Um alvo de verificação pode ser um arquivo individual (não só
        // uma pasta) -- `validate_custom_targets` aceita qualquer caminho
        // acessível. Trata esse caso diretamente em vez de assumir que
        // `target` sempre é um diretório, para não descartar silenciosamente
        // um arquivo avulso selecionado pelo usuário.
        // `symlink_metadata` (em vez de `metadata`) consulta o atributo do
        // próprio caminho sem seguir pontos de reanálise (reparse points) --
        // o mesmo mecanismo usado pelos placeholders de nuvem do OneDrive --
        // então não arrisca abrir/resolver o arquivo e disparar um download.
        let metadata = match std::fs::symlink_metadata(target) {
            Ok(metadata) => metadata,
            Err(error) => {
                tracing::warn!(
                    path = %target.display(),
                    %error,
                    "não foi possível ler metadados do alvo de verificação"
                );
                continue;
            }
        };

        if metadata.is_file() {
            if is_cloud_only_placeholder(&metadata) {
                skipped.push(SkippedCloudFile {
                    path: target.clone(),
                });
            } else {
                files.push(target.clone());
            }
        } else if metadata.is_dir() {
            walk_dir(target, &exclude_dirs, &mut files, &mut skipped);
        }
    }

    (files, skipped)
}

fn walk_dir(
    dir: &Path,
    exclude_dirs: &[PathBuf],
    files: &mut Vec<PathBuf>,
    skipped: &mut Vec<SkippedCloudFile>,
) {
    if is_excluded(dir, exclude_dirs) {
        return;
    }

    let entries = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(error) => {
            // Diretório ilegível (permissão negada, removido entre a
            // listagem e a leitura, etc.): não há um arquivo específico
            // para reportar como resultado de verificação — registrado só
            // nos logs, mesmo comportamento que o `clamscan` já tinha
            // (pular e seguir) para diretórios inacessíveis.
            tracing::warn!(
                dir = %dir.display(),
                %error,
                "não foi possível listar diretório durante a verificação"
            );
            return;
        }
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if is_excluded(&path, exclude_dirs) {
            continue;
        }

        // `DirEntry::metadata()` não segue links/reparse points, então o
        // atributo de "só na nuvem" do próprio arquivo é visto
        // corretamente, sem risco de disparar leitura do conteúdo real.
        let metadata = match entry.metadata() {
            Ok(metadata) => metadata,
            Err(error) => {
                tracing::warn!(
                    path = %path.display(),
                    %error,
                    "não foi possível ler metadados durante a verificação"
                );
                continue;
            }
        };

        if metadata.is_dir() {
            walk_dir(&path, exclude_dirs, files, skipped);
        } else if metadata.is_file() {
            if is_cloud_only_placeholder(&metadata) {
                skipped.push(SkippedCloudFile { path });
            } else {
                files.push(path);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "corvusav-test-filelist-{name}-{}",
            std::process::id()
        ));
        std::fs::create_dir_all(&dir).expect("falha ao preparar diretório de teste");
        dir
    }

    #[test]
    fn finds_regular_files_recursively() {
        let root = make_temp_dir("recursivo");
        let sub = root.join("subpasta");
        std::fs::create_dir_all(&sub).unwrap();
        std::fs::write(root.join("a.txt"), b"conteudo a").unwrap();
        std::fs::write(sub.join("b.txt"), b"conteudo b").unwrap();

        let (files, skipped) = enumerate_files(std::slice::from_ref(&root), &[]);

        std::fs::remove_dir_all(&root).ok();

        assert_eq!(skipped.len(), 0);
        assert_eq!(files.len(), 2);
        assert!(files.iter().any(|f| f.ends_with("a.txt")));
        assert!(files.iter().any(|f| f.ends_with("b.txt")));
    }

    #[test]
    fn respects_exclude_dirs() {
        let root = make_temp_dir("exclusao");
        let excluded = root.join("ignorar");
        std::fs::create_dir_all(&excluded).unwrap();
        std::fs::write(root.join("incluido.txt"), b"x").unwrap();
        std::fs::write(excluded.join("ignorado.txt"), b"x").unwrap();

        let (files, _) =
            enumerate_files(std::slice::from_ref(&root), &[excluded.to_string_lossy().to_string()]);

        std::fs::remove_dir_all(&root).ok();

        assert_eq!(files.len(), 1);
        assert!(files[0].ends_with("incluido.txt"));
    }

    #[test]
    fn missing_target_directory_yields_no_files_and_no_panic() {
        let bogus = PathBuf::from("/caminho/que/certamente/nao/existe/corvusav-teste-filelist");
        let (files, skipped) = enumerate_files(&[bogus], &[]);
        assert!(files.is_empty());
        assert!(skipped.is_empty());
    }

    #[test]
    fn single_file_target_is_scanned_directly() {
        // Regressão: um alvo de verificação personalizada pode ser um
        // arquivo avulso, não só uma pasta (`validate_custom_targets` não
        // exige diretório). Antes desta correção, `enumerate_files` assumia
        // que todo alvo era uma pasta e descartava silenciosamente um
        // arquivo avulso.
        let root = make_temp_dir("arquivo-avulso");
        let file_path = root.join("sozinho.txt");
        std::fs::write(&file_path, b"conteudo").unwrap();

        let (files, skipped) = enumerate_files(std::slice::from_ref(&file_path), &[]);

        std::fs::remove_dir_all(&root).ok();

        assert_eq!(skipped.len(), 0);
        assert_eq!(files, vec![file_path]);
    }
}
