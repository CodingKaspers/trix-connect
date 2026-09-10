//! # trix-connect
//!
//! A neutral coupling layer shared by the -trix application family. It belongs
//! to no single app: an invoicing app, a bookkeeping app, and a master-data app
//! all depend on it to talk over a stable seam, and each stays fully usable on
//! its own without the others.
//!
//! The crate carries four things and nothing app-specific:
//!
//! * [`version`] — the contract version and handshake negotiation. This is the
//!   only number that governs wire compatibility, decoupled from crate semver
//!   and from any app's own version.
//! * [`manifest`] — local discovery, so co-installed apps find each other with
//!   no configuration.
//! * [`invoice`] — the bookkeeping-neutral invoice contract that crosses the
//!   seam. No accounts, no VAT codes, no journal lines.
//! * [`vat`] / [`sink`] — the traits the bookkeeping/master-data side
//!   implements and the invoicing side calls.
//!
//! ## Versioning in one line
//!
//! Additive change (new optional field, new enum variant) bumps the minor;
//! changing how an existing field is read bumps the major. DTOs never use
//! `deny_unknown_fields` and make optional fields `#[serde(default)]`, so an
//! older reader tolerates a newer sender and vice versa within a major.

pub mod invoice;
pub mod manifest;
pub mod sink;
pub mod vat;
pub mod version;

/// The contract version this build of the crate implements. An app advertises a
/// [`version::ContractRange`], not this bare value, but this is the natural
/// upper bound for a freshly built app.
pub const CONTRACT: version::ContractVersion = version::ContractVersion::new(1, 0);

pub use invoice::{Address, Customer, Invoice, InvoiceLine, Totals};
pub use manifest::{AppManifest, capability};
pub use sink::{BookError, BookResult, BookkeepingSink, PaymentStatus, VatRateSource};
pub use vat::{VatRate, VatTreatment};
pub use version::{negotiate, ContractRange, ContractVersion};
