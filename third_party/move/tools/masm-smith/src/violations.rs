// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Decides when to break a verifier rule on purpose.
//!
//! When a rule's `valid_*` flag is `false`, generators ask [`wants_violation`] whether to break
//! it at that point, a choice drawn from the input bytes. A test records only the violations it
//! actually emits.

use crate::{
    config::{Config, DimensionFlag},
    error::Result,
};
use arbitrary::Unstructured;

/// `true` with probability `prob / 256`. Exhausted input yields `false`, so a rare choice
/// does not become certain once the bytes run out (an exhausted draw is the range minimum).
pub fn chance(u: &mut Unstructured<'_>, prob: u8) -> Result<bool> {
    let draw: u8 = u.int_in_range(0..=u8::MAX)?;
    Ok(draw > u8::MAX - prob)
}

/// Decision only: `true` when the dimension flag is `false` *and* the dice come up positive.
/// Consumes one byte from `u` iff the flag is `false`. Does not record — the caller must
/// record the dimension at the point a violation is actually emitted.
pub fn wants_violation(u: &mut Unstructured<'_>, cfg: &Config, dim: DimensionFlag) -> Result<bool> {
    if cfg.flag(dim) {
        return Ok(false);
    }
    chance(u, cfg.violation_injection_prob)
}

/// [`wants_violation`] plus recording: on `true`, pushes the dimension into `injected` so
/// the test/fuzz harness can see which violations actually fired for a given seed. Only for
/// sites where the decision always materializes as an emitted violation.
pub fn should_violate(
    u: &mut Unstructured<'_>,
    cfg: &Config,
    dim: DimensionFlag,
    injected: &mut Vec<DimensionFlag>,
) -> Result<bool> {
    let fire = wants_violation(u, cfg, dim)?;
    if fire {
        injected.push(dim);
    }
    Ok(fire)
}
