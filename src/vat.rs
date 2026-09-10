//! VAT as it crosses the seam: a *treatment* (which rate band applies), never a
//! VAT code or ledger account. The sender says "this line is reduced rate"; the
//! bookkeeping side decides which percentage and which revenue account that maps
//! to on the booking date. That split is what keeps an invoicing app free of any
//! bookkeeping knowledge.
//!
//! The rate *table* (which treatments exist and their current percentages) is
//! master data. A standalone app ships a built-in default; when coupled it syncs
//! the table from the bookkeeping/master-data side via [`VatRateSource`]. Either
//! way, what travels on an invoice line is the treatment, not the percentage.
//!
//! [`VatRateSource`]: crate::sink::VatRateSource

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

/// A rate band, not a percentage and not an account. New variants are an
/// additive (minor) change: an older reader that does not know a variant will
/// fail to deserialize it, so a sender must only use variants within the
/// negotiated contract version.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VatTreatment {
    /// High rate (NL 21%).
    Standard,
    /// Reduced rate (NL 9%).
    Reduced,
    /// Zero-rated (0%).
    Zero,
    /// Exempt from VAT.
    Exempt,
    /// Reverse-charged (btw verlegd).
    Reverse,
}

/// One row of the synced rate table: the percentage that belongs to a treatment
/// in a country over a validity window. Percentages travel as strings so they
/// stay exact decimals, never `f64`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct VatRate {
    pub treatment: VatTreatment,
    /// Exact decimal as a string, e.g. "21.00". `None` for treatments without a
    /// numeric rate (exempt, reverse).
    #[serde(default)]
    pub percentage: Option<String>,
    pub valid_from: NaiveDate,
    #[serde(default)]
    pub valid_to: Option<NaiveDate>,
    /// i18n key for the human label, never literal text.
    pub label_key: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn treatment_wire_names_are_stable() {
        assert_eq!(
            serde_json::to_string(&VatTreatment::Reduced).unwrap(),
            "\"reduced\""
        );
        assert_eq!(
            serde_json::to_string(&VatTreatment::Reverse).unwrap(),
            "\"reverse\""
        );
    }

    #[test]
    fn rate_roundtrips() {
        let r = VatRate {
            treatment: VatTreatment::Standard,
            percentage: Some("21.00".into()),
            valid_from: NaiveDate::from_ymd_opt(2019, 1, 1).unwrap(),
            valid_to: None,
            label_key: "vat.standard".into(),
        };
        let json = serde_json::to_string(&r).unwrap();
        assert_eq!(serde_json::from_str::<VatRate>(&json).unwrap(), r);
    }
}
