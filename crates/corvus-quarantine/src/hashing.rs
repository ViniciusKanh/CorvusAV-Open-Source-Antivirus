//! Hash de arquivos usado para verificar integridade antes/depois do
//! isolamento e antes de uma restauração (detecta adulteração do arquivo
//! enquanto ele está em quarentena).

use sha2::{Digest, Sha256};
use std::fs::File;
use std::io::{self, Read};
use std::path::Path;

/// Hash SHA-256 de um arquivo, lido em blocos — nunca carrega o arquivo
/// inteiro na memória de uma vez (importante para arquivos grandes).
pub fn sha256_file(path: &Path) -> io::Result<String> {
    let mut file = File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 65536];
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_content_yields_same_hash() {
        let dir = std::env::temp_dir().join(format!("corvusav-test-hash-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("arquivo.txt");
        std::fs::write(&path, b"conteudo de teste").unwrap();

        let a = sha256_file(&path).unwrap();
        let b = sha256_file(&path).unwrap();

        std::fs::remove_dir_all(&dir).ok();
        assert_eq!(a, b);
        assert_eq!(a.len(), 64, "sha256 em hexadecimal deve ter 64 caracteres");
    }
}
