//! DIAG: explicit local host assessment or scoped, read-only JSONL evidence import.

use pipboy_core::diagnostics::{
    analyze_host, analyze_ndjson, DiagnosticReport, MAX_DOCUMENT_BYTES,
};
use std::error::Error;
use std::fs::File;
use std::io::Read;

const USAGE: &str =
    "usage: pipboy diag <host [--json] | ingest FILE --asset ID --confirm-authorized [--json]>";

fn print_report(report: &DiagnosticReport, as_json: bool) -> Result<(), Box<dyn Error>> {
    if as_json {
        println!("{}", serde_json::to_string_pretty(report)?);
    } else {
        println!("{}", report.render_text());
    }
    Ok(())
}

pub fn run(args: Vec<String>) -> Result<(), Box<dyn Error>> {
    let mut args = args.into_iter();
    match args.next().as_deref() {
        Some("host") => {
            let as_json = match (args.next().as_deref(), args.next()) {
                (None, None) => false,
                (Some("--json"), None) => true,
                _ => return Err(USAGE.into()),
            };
            // Reuse the existing host.read lifecycle; do not request device/network grants.
            let snapshot = crate::status::collect()?;
            let report = analyze_host(&snapshot)?;
            print_report(&report, as_json)
        }
        Some("ingest") => {
            let path = args.next().ok_or(USAGE)?;
            let mut asset: Option<String> = None;
            let mut acknowledged = false;
            let mut as_json = false;
            while let Some(arg) = args.next() {
                match arg.as_str() {
                    "--asset" if asset.is_none() => asset = Some(args.next().ok_or(USAGE)?),
                    "--confirm-authorized" if !acknowledged => acknowledged = true,
                    "--json" if !as_json => as_json = true,
                    _ => return Err(USAGE.into()),
                }
            }
            let asset = asset.ok_or(USAGE)?;
            if !acknowledged {
                return Err("DIAG import requires --confirm-authorized: attest that the supplied evidence is within your authorized scope".into());
            }
            let file = File::open(&path)?;
            if !file.metadata()?.is_file() {
                return Err("DIAG only imports a specifically named regular file".into());
            }
            // Bound memory even if the file changes after metadata inspection.
            let mut input = String::new();
            file.take((MAX_DOCUMENT_BYTES + 1) as u64)
                .read_to_string(&mut input)?;
            let report = analyze_ndjson(&input, &asset, acknowledged)?;
            print_report(&report, as_json)
        }
        _ => Err(USAGE.into()),
    }
}
