//! Optional, read-only Better BibTeX attachment lookup on the local machine.

use serde_json::{Value, json};
use std::{
    io,
    path::PathBuf,
    process::{Command, Stdio},
    sync::mpsc::{self, Receiver},
};

const ENDPOINT: &str = "http://127.0.0.1:23119/better-bibtex/json-rpc";
const MAX_RESPONSE: usize = 16 * 1024 * 1024;
pub(in crate::app) type Lookup = Receiver<Result<Vec<PathBuf>, String>>;

pub(super) fn start(key: &str) -> Result<Lookup, String> {
    let key = key.to_owned();
    let (tx, rx) = mpsc::channel();
    // Do not access the user's live Zotero library during automated tests.
    if cfg!(test) {
        let _ = tx.send(Ok(Vec::new()));
        return Ok(rx);
    }
    drop(
        std::thread::Builder::new()
            .name("zettui-zotero-attachments".into())
            .spawn(move || {
                let _ = tx.send(lookup(&key));
            })
            .map_err(|err| format!("Cannot start Zotero lookup: {err}"))?,
    );
    Ok(rx)
}

fn lookup(key: &str) -> Result<Vec<PathBuf>, String> {
    let request = json!({"jsonrpc":"2.0", "id":1, "method":"item.attachments", "params":[key]});
    // curl is optional. Ignore user curlrc and proxies; never follow redirects.
    // Pass JSON as a literal argument, not through a shell.
    let output = Command::new("curl")
        .args([
            "--disable",
            "--noproxy",
            "*",
            "--silent",
            "--show-error",
            "--fail",
            "--connect-timeout",
            "1",
            "--max-time",
            "5",
            "--max-filesize",
            "16777216",
            "--header",
            "Content-Type: application/json",
            "--header",
            "Accept: application/json",
            "--data-binary",
        ])
        .arg(request.to_string())
        .arg(ENDPOINT)
        .stdin(Stdio::null())
        .output()
        .map_err(|err| {
            if err.kind() == io::ErrorKind::NotFound {
                "Install curl for live Zotero PDF lookup.".into()
            } else {
                format!("Cannot run curl: {err}")
            }
        })?;
    if !output.status.success() {
        return Err(match output.status.code() {
            Some(7) => "Zotero unavailable; open Zotero and check Better BibTeX.".into(),
            Some(28) => "Zotero lookup timed out; press r to retry.".into(),
            Some(22) => "Zotero HTTP error; check Better BibTeX and local API settings.".into(),
            Some(63) => "Zotero attachment response too large.".into(),
            _ => format!("Zotero lookup failed ({})", output.status),
        });
    }
    if output.stdout.len() > MAX_RESPONSE {
        return Err("Zotero attachment response too large.".into());
    }
    parse_response(&output.stdout)
}

fn parse_response(raw: &[u8]) -> Result<Vec<PathBuf>, String> {
    let value: Value =
        serde_json::from_slice(raw).map_err(|err| format!("Invalid Zotero JSON: {err}"))?;
    if let Some(error) = value.get("error").filter(|v| !v.is_null()) {
        let message = error
            .get("message")
            .and_then(Value::as_str)
            .unwrap_or("Unknown RPC error");
        return Err(format!(
            "Zotero: {}",
            message.chars().take(160).collect::<String>()
        ));
    }
    let items = value
        .get("result")
        .and_then(Value::as_array)
        .ok_or_else(|| "Invalid Zotero attachment result.".to_owned())?;
    let mut paths = Vec::new();
    for item in items {
        let Some(raw) = item.get("path").and_then(Value::as_str) else {
            continue;
        };
        let path = PathBuf::from(raw);
        if path.is_absolute()
            && !raw.contains('\0')
            && path
                .extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("pdf"))
            && !paths.contains(&path)
        {
            paths.push(path);
        }
    }
    Ok(paths)
}

/// Merge exported and live paths, retaining the selected index and list order.
pub(super) fn merge_paths(paths: &mut Vec<PathBuf>, incoming: Vec<PathBuf>) {
    for path in incoming {
        if !paths.contains(&path) {
            paths.push(path);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rpc_filters_and_deduplicates_local_pdf_paths() {
        let raw = br#"{"jsonrpc":"2.0","id":1,"result":[
            {"path":"/Zotero/storage/A/First paper.pdf","open":"zotero://open-pdf/library/items/A"},
            {"path":"/Zotero/storage/B/Second.PDF"},{"path":"/Zotero/storage/A/First paper.pdf"},
            {"path":false},{"path":null},{"path":"https://example.org/x.pdf"},
            {"path":"relative.pdf"},{"path":"/tmp/page.html"}]}"#;
        assert_eq!(
            parse_response(raw).unwrap(),
            vec![
                PathBuf::from("/Zotero/storage/A/First paper.pdf"),
                PathBuf::from("/Zotero/storage/B/Second.PDF"),
            ]
        );
    }

    #[test]
    fn rpc_distinguishes_empty_results_and_errors() {
        assert!(parse_response(br#"{"result":[]}"#).unwrap().is_empty());
        assert!(
            parse_response(br#"{"error":{"code":-32602,"message":"key not found"}}"#)
                .unwrap_err()
                .contains("key not found")
        );
        assert!(parse_response(b"not JSON").is_err());
        assert!(parse_response(br#"{"result":null}"#).is_err());
    }

    #[test]
    fn merge_keeps_exported_paths_and_selection_order() {
        let a = PathBuf::from("/tmp/a.pdf");
        let b = PathBuf::from("/tmp/b.pdf");
        let mut paths = vec![a.clone()];
        merge_paths(&mut paths, vec![a.clone(), b.clone(), b.clone()]);
        assert_eq!(paths, vec![a, b]);
    }
}
