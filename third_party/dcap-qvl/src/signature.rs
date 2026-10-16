//! Audited [`EcdsaSigEncoder`] implementation backed by `der`.
//!
//! Selected by [`crate::configs::RingConfig`] / [`crate::configs::RustCryptoConfig`]
//! / [`crate::configs::DefaultConfig`].

use alloc::vec::Vec;
use anyhow::{Context, Result};

use crate::config::EcdsaSigEncoder;

/// Audited default [`EcdsaSigEncoder`] implementation, built on `der`.
///
/// Stateless zero-sized marker type.
pub struct DerSigEncoder;

impl EcdsaSigEncoder for DerSigEncoder {
    fn encode_ecdsa_sig(r: &[u8], s: &[u8]) -> Result<Vec<u8>> {
        let r_int = der::asn1::UintRef::new(r).context("Failed to create r INTEGER")?;
        let s_int = der::asn1::UintRef::new(s).context("Failed to create s INTEGER")?;
        let sequence = alloc::vec![r_int, s_int];
        // Capacity: SEQUENCE header (≤4) + 2 × (INTEGER header (≤4) + payload + 1 sign byte)
        let cap = 8usize
            .checked_add(r.len())
            .and_then(|n| n.checked_add(s.len()))
            .and_then(|n| n.checked_add(2))
            .context("encode_ecdsa_sig capacity overflow")?;
        let mut buf = alloc::vec![0u8; cap];
        let mut writer = der::SliceWriter::new(&mut buf);
        writer
            .encode(&sequence)
            .context("Failed to encode ECDSA signature sequence")?;
        Ok(writer
            .finish()
            .context("Failed to finalize ECDSA signature sequence")?
            .to_vec())
    }
}
