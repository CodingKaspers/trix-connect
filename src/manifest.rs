//! Local discovery. Each -trix app writes a small manifest to a shared directory
//! at install/boot; every app reads that directory on boot and so "sees" its
//! siblings on the same machine. Coupling then needs no configuration: an
//! invoicing app that finds a bookkeeping manifest can offer to couple.
//!
//! This is discovery only. Deliberate coupling to a remote/cloud instance is a
//! separate manual path (a base URL plus a key) and does not go through here.

use std::fs;
use std::io;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::version::ContractRange;

/// Capability strings advertised in a manifest. Presence gates optional calls,
/// so an older peer missing one degrades gracefully instead of erroring.
pub mod capability {
    pub const BOOK_SALES_INVOICE: &str = "book_sales_invoice";
    pub const PAYMENT_STATUS: &str = "payment_status";
    pub const VAT_RATES: &str = "vat_rates";
}

/// What one app publishes about itself for local discovery.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppManifest {
    /// Short app key, e.g. "ledgerix" or "facturix". Also the file stem.
    pub app: String,
    pub app_version: String,
    /// Where to reach this instance locally, e.g. "http://localhost:8080".
    pub base_url: String,
    /// The contract band this app speaks.
    pub contract: ContractRange,
    pub capabilities: Vec<String>,
}

impl AppManifest {
    pub fn has_capability(&self, cap: &str) -> bool {
        self.capabilities.iter().any(|c| c == cap)
    }
}

/// The shared manifest directory. Overridable via `TRIX_MANIFEST_DIR` (used by
/// tests and by non-default deployments); otherwise a per-platform default that
/// matches where the installers already place shared -trix data.
pub fn manifest_dir() -> PathBuf {
    if let Ok(dir) = std::env::var("TRIX_MANIFEST_DIR") {
        return PathBuf::from(dir);
    }
    #[cfg(windows)]
    {
        let base = std::env::var("ProgramData")
            .unwrap_or_else(|_| r"C:\ProgramData".to_string());
        PathBuf::from(base).join("Trix")
    }
    #[cfg(not(windows))]
    {
        PathBuf::from("/var/lib/trix")
    }
}

/// Write this app's manifest as `<dir>/<app>.json`, creating the directory if
/// needed. Call at boot so the advertised `base_url`/version are current.
pub fn write_manifest(m: &AppManifest) -> io::Result<()> {
    let dir = manifest_dir();
    fs::create_dir_all(&dir)?;
    let path = dir.join(format!("{}.json", m.app));
    let json = serde_json::to_vec_pretty(m)?;
    fs::write(path, json)
}

/// Read every readable manifest in the shared directory. A missing directory is
/// not an error (nothing is installed yet); unreadable or malformed entries are
/// skipped so one bad file does not blind discovery.
pub fn read_all() -> Vec<AppManifest> {
    let dir = manifest_dir();
    let entries = match fs::read_dir(&dir) {
        Ok(e) => e,
        Err(_) => return Vec::new(),
    };
    let mut out = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        if let Ok(bytes) = fs::read(&path) {
            if let Ok(m) = serde_json::from_slice::<AppManifest>(&bytes) {
                out.push(m);
            }
        }
    }
    out
}

/// Find a specific sibling by app key, if present.
pub fn find(app: &str) -> Option<AppManifest> {
    read_all().into_iter().find(|m| m.app == app)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::version::ContractVersion;

    // Serialize manifest-directory access so parallel tests don't fight over the
    // shared env var and temp dir.
    static LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    fn sample(app: &str) -> AppManifest {
        AppManifest {
            app: app.into(),
            app_version: "0.96.0".into(),
            base_url: "http://localhost:8080".into(),
            contract: ContractRange::new(
                ContractVersion::new(1, 0),
                ContractVersion::new(1, 2),
            ),
            capabilities: vec![
                capability::BOOK_SALES_INVOICE.into(),
                capability::VAT_RATES.into(),
            ],
        }
    }

    #[test]
    fn write_then_discover_roundtrips() {
        let _g = LOCK.lock().unwrap();
        let tmp = std::env::temp_dir().join(format!("trix-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&tmp);
        std::env::set_var("TRIX_MANIFEST_DIR", &tmp);

        write_manifest(&sample("ledgerix")).unwrap();
        let found = find("ledgerix").expect("manifest should be discoverable");
        assert_eq!(found, sample("ledgerix"));
        assert!(found.has_capability(capability::VAT_RATES));
        assert!(!found.has_capability(capability::PAYMENT_STATUS));

        std::env::remove_var("TRIX_MANIFEST_DIR");
        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn missing_directory_is_empty_not_error() {
        let _g = LOCK.lock().unwrap();
        let tmp = std::env::temp_dir().join(format!("trix-none-{}", std::process::id()));
        let _ = fs::remove_dir_all(&tmp);
        std::env::set_var("TRIX_MANIFEST_DIR", &tmp);
        assert!(read_all().is_empty());
        std::env::remove_var("TRIX_MANIFEST_DIR");
    }
}
