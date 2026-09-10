//! Contract versioning and handshake negotiation.
//!
//! The contract version is deliberately decoupled from both the crate's
//! semver and any app's own version. Only this number decides whether two
//! apps can talk over the seam. The rule that governs every change:
//! additive (a new optional field, a new enum variant with a backward
//! default) bumps the minor; anything that changes how an existing field is
//! read bumps the major. Different majors are never wire-compatible, so the
//! negotiation below refuses them rather than guessing.

use std::fmt;
use std::str::FromStr;

use serde::{de, Deserialize, Deserializer, Serialize, Serializer};

/// A `major.minor` contract version. Patch is intentionally absent: a patch
/// would never change the wire, so it has no meaning here.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct ContractVersion {
    pub major: u16,
    pub minor: u16,
}

impl ContractVersion {
    pub const fn new(major: u16, minor: u16) -> Self {
        Self { major, minor }
    }
}

impl fmt::Display for ContractVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}", self.major, self.minor)
    }
}

impl FromStr for ContractVersion {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (maj, min) = s
            .split_once('.')
            .ok_or_else(|| format!("invalid contract version: {s}"))?;
        let major = maj
            .parse()
            .map_err(|_| format!("invalid major in contract version: {s}"))?;
        let minor = min
            .parse()
            .map_err(|_| format!("invalid minor in contract version: {s}"))?;
        Ok(Self { major, minor })
    }
}

// Wire form is the bare string "1.0", not a nested object, so manifests and
// DTOs stay readable.
impl Serialize for ContractVersion {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for ContractVersion {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let s = String::deserialize(d)?;
        s.parse().map_err(de::Error::custom)
    }
}

/// The inclusive band of contract versions an app can speak, advertised in its
/// manifest. Not a single version: a running app usually supports several
/// minors so the other side has room to be older or newer.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContractRange {
    pub min: ContractVersion,
    pub max: ContractVersion,
}

impl ContractRange {
    pub const fn new(min: ContractVersion, max: ContractVersion) -> Self {
        Self { min, max }
    }

    /// A range that supports exactly one version.
    pub const fn exact(v: ContractVersion) -> Self {
        Self { min: v, max: v }
    }
}

/// Pick the highest version both sides can speak, or `None` when their bands
/// do not overlap (typically a major gap). Because versions order
/// lexicographically by `(major, minor)`, a cross-major situation collapses to
/// an empty overlap on its own, so the caller gets a clean refusal instead of
/// a silent mismatch.
pub fn negotiate(a: ContractRange, b: ContractRange) -> Option<ContractVersion> {
    let lo = a.min.max(b.min);
    let hi = a.max.min(b.max);
    (lo <= hi).then_some(hi)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_and_renders() {
        let v: ContractVersion = "1.2".parse().unwrap();
        assert_eq!(v, ContractVersion::new(1, 2));
        assert_eq!(v.to_string(), "1.2");
    }

    #[test]
    fn orders_by_major_then_minor() {
        assert!(ContractVersion::new(1, 9) < ContractVersion::new(2, 0));
        assert!(ContractVersion::new(1, 2) > ContractVersion::new(1, 1));
    }

    #[test]
    fn negotiate_picks_highest_common_minor() {
        let a = ContractRange::new(ContractVersion::new(1, 0), ContractVersion::new(2, 3));
        let b = ContractRange::new(ContractVersion::new(1, 5), ContractVersion::new(1, 9));
        assert_eq!(negotiate(a, b), Some(ContractVersion::new(1, 9)));
    }

    #[test]
    fn negotiate_refuses_disjoint_majors() {
        let a = ContractRange::exact(ContractVersion::new(1, 2));
        let b = ContractRange::exact(ContractVersion::new(2, 0));
        assert_eq!(negotiate(a, b), None);
    }

    #[test]
    fn version_survives_json_roundtrip() {
        let v = ContractVersion::new(1, 4);
        let json = serde_json::to_string(&v).unwrap();
        assert_eq!(json, "\"1.4\"");
        assert_eq!(serde_json::from_str::<ContractVersion>(&json).unwrap(), v);
    }
}
