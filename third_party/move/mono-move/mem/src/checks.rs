// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Runtime memory checks: the tier they run at, how they fail, and the
//! [`mem_check!`](crate::mem_check) macro that attaches one to a site.
//!
//! A check never changes what a valid program computes; it can only fail on
//! a VM bug. The production build compiles no check code at all. Which tier
//! a binary has is decided when this crate is built, from the
//! `MONO_MOVE_MEM_CHECKS` environment variable (see `build.rs`), and is
//! exposed as [`CHECK_TIER`] so a binary can report it.
//!
//! Failure: [`fault`] records the [`MemFault`] in a thread-local slot
//! ([`take_last_fault`]) and unwinds with it as the panic payload. The
//! transaction boundary (test harnesses today) turns that into an invariant
//! violation. Checks are not metered and never alter gas.

use std::{cell::Cell, fmt, panic};

/// Which checks a binary was built with.
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub enum CheckTier {
    /// No check code is compiled.
    Prod,
    /// O(1) checks on the access path.
    Hardened,
    /// Hardened plus checks that may do more than O(1) work per access.
    Exhaustive,
}

/// The tier this build of the crate was compiled with.
pub const CHECK_TIER: CheckTier = if cfg!(mono_move_mem_checks = "exhaustive") {
    CheckTier::Exhaustive
} else if cfg!(mono_move_mem_checks = "hardened") {
    CheckTier::Hardened
} else {
    CheckTier::Prod
};

/// Whether hardened-tier checks are compiled in. A constant, so a branch on
/// it folds away.
pub const HARDENED: bool = matches!(CHECK_TIER, CheckTier::Hardened | CheckTier::Exhaustive);

/// Whether exhaustive-tier checks are compiled in.
pub const EXHAUSTIVE: bool = matches!(CHECK_TIER, CheckTier::Exhaustive);

/// What a failed check found.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum MemFaultKind {
    /// A typed access at an address not aligned for the type.
    Misaligned,
    /// An access through the null address.
    Null,
    /// Address arithmetic wrapped around the address space.
    AddressWrap,
    /// A non-overlapping copy or swap whose ranges overlap.
    Overlap,
    /// A boolean slot holding a byte other than 0 or 1.
    InvalidBool,
    /// A fat reference's offset overflowed or exceeds its representable range.
    FatRefRange,
    /// An object pointer that is null or not `MAX_ALIGN`-aligned.
    BadObjectPointer,
    /// An object header whose size is zero, unaligned, or marks a forwarded
    /// object outside garbage collection.
    BadHeader,
    /// A vector element outside the object's payload.
    VectorOutOfPayload,
    /// A field access outside the object's payload.
    FieldOutOfPayload,
    /// A frame pointer that is not `MAX_ALIGN`-aligned.
    BadFramePointer,
    /// Frame metadata that cannot have been written by the call protocol.
    BadFrameMeta,
}

/// A failed memory check: what was found and where.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct MemFault {
    pub kind: MemFaultKind,
    /// The address the check was evaluating.
    pub addr: usize,
    /// Access width or object size, whichever the check was about; 0 if none.
    pub size: usize,
    /// Required alignment or the offending offset, whichever the check was
    /// about; 0 if none.
    pub extra: usize,
}

impl MemFault {
    #[inline(always)]
    pub const fn new(kind: MemFaultKind, addr: usize, size: usize, extra: usize) -> MemFault {
        MemFault {
            kind,
            addr,
            size,
            extra,
        }
    }
}

impl fmt::Display for MemFault {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "memory check failed: {:?} at {:#x} (size {}, extra {:#x})",
            self.kind, self.addr, self.size, self.extra
        )
    }
}

thread_local! {
    static LAST_FAULT: Cell<Option<MemFault>> = const { Cell::new(None) };
}

/// Reports a failed check: records it for [`take_last_fault`] and unwinds
/// with the fault as the panic payload.
#[cold]
#[inline(never)]
pub fn fault(f: MemFault) -> ! {
    LAST_FAULT.with(|slot| slot.set(Some(f)));
    panic::panic_any(f)
}

/// The most recent fault on this thread, if any, clearing it.
pub fn take_last_fault() -> Option<MemFault> {
    LAST_FAULT.with(|slot| slot.take())
}

/// Attaches a check to a site. The first argument is the tier at which the
/// check is a real check; below that tier it is a `debug_assert!`
/// (`hardened`) or nothing (`exhaustive`). `$cond` is only evaluated when
/// the check is active; `$fault` only when it fails.
#[macro_export]
macro_rules! mem_check {
    (hardened, $cond:expr, $fault:expr $(,)?) => {
        if $crate::checks::HARDENED {
            if !$cond {
                $crate::checks::fault($fault);
            }
        } else {
            debug_assert!($cond, "{}", $fault);
        }
    };
    (exhaustive, $cond:expr, $fault:expr $(,)?) => {
        if $crate::checks::EXHAUSTIVE {
            if !$cond {
                $crate::checks::fault($fault);
            }
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fault_records_and_unwinds() {
        let f = MemFault::new(MemFaultKind::Null, 0, 8, 0);
        let r = panic::catch_unwind(|| fault(f));
        let payload = r.expect_err("fault must unwind");
        assert_eq!(payload.downcast_ref::<MemFault>(), Some(&f));
        assert_eq!(take_last_fault(), Some(f));
        assert_eq!(take_last_fault(), None);
    }

    #[test]
    fn tier_matches_build_env() {
        let expected = match option_env!("MONO_MOVE_MEM_CHECKS") {
            None | Some("") | Some("none") => CheckTier::Prod,
            Some("hardened") => CheckTier::Hardened,
            Some("exhaustive") => CheckTier::Exhaustive,
            Some(other) => panic!("unexpected MONO_MOVE_MEM_CHECKS={other}"),
        };
        assert_eq!(CHECK_TIER, expected);
    }
}
