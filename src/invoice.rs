//! The invoice contract: the data shape that crosses the seam from an invoicing
//! app to a bookkeeping app. It is deliberately bookkeeping-neutral. There are
//! no ledger accounts, no VAT codes, no journal lines here. The sender describes
//! an invoice; the receiver maps it to its own bookkeeping.
//!
//! Two rules keep this forward-compatible, matching the versioning model in
//! [`crate::version`]:
//! * no `deny_unknown_fields`, so a newer sender may include fields an older
//!   reader does not know, and the reader ignores them;
//! * every optional field is `#[serde(default)]`, so an older sender that omits
//!   a newer field still deserializes.
//!
//! All money travels as decimal strings, never `f64`.

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

use crate::vat::VatTreatment;
use crate::version::ContractVersion;

/// A complete sales invoice handed over for booking.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Invoice {
    /// The negotiated contract version this payload is written against. Lets the
    /// receiver read it the right way even if it supports several.
    pub contract: ContractVersion,
    /// Which app produced it, e.g. "facturix".
    pub source: String,
    /// The sender's own stable id for this invoice. This is the idempotency key:
    /// booking the same `external_id` twice must not double-book.
    pub external_id: String,
    /// The sender's human-visible invoice number.
    pub invoice_number: String,
    pub issue_date: NaiveDate,
    #[serde(default)]
    pub due_date: Option<NaiveDate>,
    /// ISO-4217, e.g. "EUR".
    pub currency: String,
    pub customer: Customer,
    pub lines: Vec<InvoiceLine>,
    /// Totals computed by the sender. The receiver verifies rather than trusts
    /// them (see [`Totals`]).
    pub totals: Totals,
    #[serde(default)]
    pub notes: Option<String>,
    #[serde(default)]
    pub reference: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Customer {
    /// Shared master-data id (e.g. a Referix relation) when coupled; `None` when
    /// the sender owns the customer standalone.
    #[serde(default)]
    pub external_id: Option<String>,
    pub name: String,
    #[serde(default)]
    pub vat_number: Option<String>,
    #[serde(default)]
    pub address: Option<Address>,
    #[serde(default)]
    pub email: Option<String>,
    /// ISO-3166 alpha-2; the receiver assumes "NL" when absent.
    #[serde(default)]
    pub country: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Address {
    #[serde(default)]
    pub line1: Option<String>,
    #[serde(default)]
    pub line2: Option<String>,
    #[serde(default)]
    pub postal_code: Option<String>,
    #[serde(default)]
    pub city: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct InvoiceLine {
    pub description: String,
    /// Decimal as a string.
    pub quantity: String,
    /// "stuk", "uur", ... Free-form; the receiver does not interpret it.
    #[serde(default)]
    pub unit: Option<String>,
    /// Unit price excluding VAT, decimal as a string.
    pub unit_price: String,
    /// Which rate band applies. The receiver maps it to its own VAT code and
    /// revenue account.
    pub vat: VatTreatment,
    /// Optional steer toward a revenue grouping. The receiver is free to ignore
    /// it; it must never be required to honour it.
    #[serde(default)]
    pub revenue_hint: Option<String>,
}

/// Sender-computed totals, decimal strings. They ride along so the receiver can
/// verify its own mapping against them and refuse a payload whose rounding does
/// not reconcile, rather than post a crooked booking.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Totals {
    pub net: String,
    pub vat: String,
    pub gross: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Invoice {
        Invoice {
            contract: ContractVersion::new(1, 0),
            source: "facturix".into(),
            external_id: "FX-2026-0001".into(),
            invoice_number: "2026-0001".into(),
            issue_date: NaiveDate::from_ymd_opt(2026, 9, 10).unwrap(),
            due_date: None,
            currency: "EUR".into(),
            customer: Customer {
                external_id: None,
                name: "Voorbeeld BV".into(),
                vat_number: Some("NL001234567B01".into()),
                address: None,
                email: None,
                country: None,
            },
            lines: vec![InvoiceLine {
                description: "Advies".into(),
                quantity: "2".into(),
                unit: Some("uur".into()),
                unit_price: "95.00".into(),
                vat: VatTreatment::Standard,
                revenue_hint: None,
            }],
            totals: Totals {
                net: "190.00".into(),
                vat: "39.90".into(),
                gross: "229.90".into(),
            },
            notes: None,
            reference: None,
        }
    }

    #[test]
    fn roundtrips() {
        let inv = sample();
        let json = serde_json::to_string(&inv).unwrap();
        assert_eq!(serde_json::from_str::<Invoice>(&json).unwrap(), inv);
    }

    #[test]
    fn unknown_fields_are_ignored_for_forward_compat() {
        // A newer sender adds a field this build has never heard of.
        let json = r#"{
            "contract": "1.0",
            "source": "facturix",
            "external_id": "FX-1",
            "invoice_number": "1",
            "issue_date": "2026-09-10",
            "currency": "EUR",
            "customer": { "name": "X" },
            "lines": [],
            "totals": { "net": "0.00", "vat": "0.00", "gross": "0.00" },
            "loyalty_points": 42
        }"#;
        let inv: Invoice = serde_json::from_str(json).unwrap();
        assert_eq!(inv.external_id, "FX-1");
        assert!(inv.customer.vat_number.is_none());
    }
}
