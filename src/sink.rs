//! The traits that sit on the seam.
//!
//! A bookkeeping app implements [`BookkeepingSink`] (and, as master-data holder,
//! [`VatRateSource`]). An invoicing app calls them through a connector. The
//! traits are the only thing either side needs to know about the other.

use async_trait::async_trait;
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

use crate::invoice::Invoice;
use crate::vat::VatRate;
use crate::version::ContractRange;

/// Where an invoice goes to be booked and have its payment tracked.
#[async_trait]
pub trait BookkeepingSink {
    /// Book a sales invoice. Idempotent on [`Invoice::external_id`]: a repeat of
    /// an already-booked invoice returns the existing result with
    /// [`BookResult::already_booked`] set, instead of posting twice.
    async fn book_sales_invoice(&self, inv: &Invoice) -> Result<BookResult, BookError>;

    /// The payment state of a previously booked invoice, keyed by its
    /// `external_id`.
    async fn payment_status(&self, external_id: &str) -> Result<PaymentStatus, BookError>;

    /// The contract band this sink speaks, for the handshake.
    fn contract_range(&self) -> ContractRange;
}

/// The master-data side of VAT rates. Split from [`BookkeepingSink`] so it can
/// move to a dedicated master-data app later without touching the caller.
#[async_trait]
pub trait VatRateSource {
    /// The rate table for a country, valid on a date. The caller receives
    /// treatment + percentage only, never accounts.
    async fn vat_rates(
        &self,
        country: &str,
        on_date: NaiveDate,
    ) -> Result<Vec<VatRate>, BookError>;
}

/// The outcome of booking an invoice. Travels back over the wire as the
/// response, so it carries serde derives.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BookResult {
    /// The receiver's own transaction id.
    pub booking_ref: String,
    /// The document/sequence number the receiver assigned.
    pub document_number: String,
    /// True when this call matched an already-booked `external_id`.
    pub already_booked: bool,
}

/// Wire form is internally tagged on `status`: `{"status":"open"}`,
/// `{"status":"partial","paid":"50.00"}`, `{"status":"paid"}`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum PaymentStatus {
    Open,
    /// Partly paid; `paid` is a decimal string.
    Partial { paid: String },
    Paid,
}

/// Failure modes on the seam. These map to i18n keys at the app layer; the
/// variants here are neutral and carry no display text.
#[derive(Clone, Debug, thiserror::Error)]
pub enum BookError {
    /// No common contract version; `theirs`/`ours` are the advertised bands.
    #[error("contract mismatch: theirs={theirs}, ours={ours}")]
    ContractMismatch { theirs: String, ours: String },
    /// Sender totals do not reconcile with the receiver's mapping.
    #[error("totals do not reconcile")]
    TotalsMismatch,
    /// The target period is closed for mutations.
    #[error("period is closed")]
    ClosedPeriod,
    /// The document number collides with an existing one.
    #[error("duplicate document number")]
    DuplicateNumber,
    /// Transport/connection failure; carries a detail string for logging.
    #[error("transport error: {0}")]
    Transport(String),
}
