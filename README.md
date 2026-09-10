# trix-connect

Neutral coupling layer for the -trix application family. It belongs to no single
app: an invoicing app (Facturix), a bookkeeping app (Ledgerix), and a
master-data app (Referix) all depend on it to talk over a stable seam, while each
stays fully usable standalone.

The crate carries four things and nothing app-specific:

- **`version`** — contract version + handshake negotiation. The only number that
  governs wire compatibility, decoupled from crate semver and each app's own
  version.
- **`manifest`** — local discovery, so co-installed apps find each other with no
  configuration.
- **`invoice`** — the bookkeeping-neutral invoice contract. No ledger accounts,
  no VAT codes, no journal lines: the sender describes an invoice, the receiver
  maps it to its own bookkeeping.
- **`vat` / `sink`** — the traits the bookkeeping/master-data side implements
  (`BookkeepingSink`, `VatRateSource`) and the invoicing side calls.

## Versioning rule

Additive change (new optional field, new enum variant with a backward default)
bumps the **minor**; changing how an existing field is read bumps the **major**.
Different majors are never wire-compatible, and `negotiate()` refuses them rather
than guessing. DTOs never use `deny_unknown_fields` and make optional fields
`#[serde(default)]`, so an older reader tolerates a newer sender and vice versa
within a major.

Current contract version: **1.0**.
