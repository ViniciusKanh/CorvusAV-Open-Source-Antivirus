//! Classificação de resultados brutos de mecanismos em vereditos e
//! detecções do domínio, preservando evidências.

use corvus_clamav::{RawScanEntry, RawVerdict};
use corvus_core::{Detection, DetectionId, ScanResultId, Verdict};

/// Converte uma entrada bruta do ClamAV em um veredito do domínio e, quando
/// aplicável, em uma detecção com evidência. Nunca classifica um arquivo com
/// erro de leitura/execução como `Clean`.
pub fn classify_entry(
    entry: &RawScanEntry,
    engine_name: &str,
    engine_version: Option<&str>,
    scan_result_id: ScanResultId,
) -> (Verdict, Vec<Detection>) {
    match &entry.verdict {
        RawVerdict::Clean => (Verdict::Clean, Vec::new()),
        RawVerdict::Infected { signature } => {
            let detection = Detection {
                id: DetectionId::new(),
                scan_result_id,
                engine_name: engine_name.to_string(),
                engine_version: engine_version.map(str::to_string),
                signature_id: signature.clone(),
                metadata: serde_json::json!({ "origem": "clamscan" }),
            };
            (Verdict::Malicious, vec![detection])
        }
        RawVerdict::Error { .. } => (Verdict::Error, Vec::new()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clean_entry_has_no_detections() {
        let entry = RawScanEntry { path: "a.txt".into(), verdict: RawVerdict::Clean };
        let (verdict, detections) =
            classify_entry(&entry, "ClamAV", Some("1.3.0"), ScanResultId::new());
        assert_eq!(verdict, Verdict::Clean);
        assert!(detections.is_empty());
    }

    #[test]
    fn infected_entry_produces_malicious_verdict_with_evidence() {
        let entry = RawScanEntry {
            path: "eicar.com".into(),
            verdict: RawVerdict::Infected { signature: "Win.Test.EICAR_HDB-1".into() },
        };
        let (verdict, detections) =
            classify_entry(&entry, "ClamAV", Some("1.3.0"), ScanResultId::new());
        assert_eq!(verdict, Verdict::Malicious);
        assert_eq!(detections.len(), 1);
        assert_eq!(detections[0].signature_id, "Win.Test.EICAR_HDB-1");
    }

    #[test]
    fn error_entry_is_never_classified_as_clean() {
        let entry = RawScanEntry {
            path: "bloqueado.dat".into(),
            verdict: RawVerdict::Error { message: "Permission denied".into() },
        };
        let (verdict, detections) =
            classify_entry(&entry, "ClamAV", None, ScanResultId::new());
        assert_eq!(verdict, Verdict::Error);
        assert!(detections.is_empty());
    }
}
