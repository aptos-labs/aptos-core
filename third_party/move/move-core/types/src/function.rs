// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Serialize};
use std::fmt;

/// A `ClosureMask` is a value which determines how to distinguish those function arguments
/// which are captured and which are not when a closure is constructed. For instance,
/// with `_` representing an omitted argument, the mask for `f(a,_,b,_)` would have the argument
/// at index 0 and at index 2 captured. The mask can be used to transform lists of types.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
#[cfg_attr(any(test, feature = "fuzzing"), derive(proptest_derive::Arbitrary))]
#[cfg_attr(any(test, feature = "fuzzing"), proptest(no_params))]
#[cfg_attr(
    feature = "fuzzing",
    derive(arbitrary::Arbitrary),
    derive(dearbitrary::Dearbitrary)
)]
pub struct ClosureMask(u64);

impl fmt::Display for ClosureMask {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:b}", self.0)
    }
}

impl ClosureMask {
    /// The maximal number of arguments which can be handled by a closure mask.
    /// A captured argument's position in the argument list must be lower than
    /// this number. Notice that this property is implicit in the bytecode:
    /// a PACK_CLOSURE instruction will never pop more arguments from the
    /// stack than this number.
    pub const MAX_ARGS: usize = 64;

    pub fn new(mask: u64) -> Self {
        Self(mask)
    }

    pub fn bits(&self) -> u64 {
        self.0
    }

    /// Apply a closure mask to a list of elements, returning only those
    /// where position `i` is set in the mask (if `collect_captured` is true) or not
    /// set (otherwise).
    pub fn extract<'a, T>(
        &self,
        values: impl IntoIterator<Item = &'a T>,
        collect_captured: bool,
    ) -> Vec<&'a T> {
        let mut mask = self.0;
        values
            .into_iter()
            .filter(|_| {
                let set = mask & 0x1 != 0;
                mask >>= 1;
                set && collect_captured || !set && !collect_captured
            })
            .collect()
    }

    /// Compose two lists of elements into one based on the given mask such that the
    /// following holds:
    /// ```ignore
    ///   mask.compose(mask.extract(v, true), mask.extract(v, false)) == v
    /// ```
    /// This returns `None` if the provided lists are inconsistent w.r.t the mask
    /// and cannot be composed. This should not happen in verified code, but
    /// a caller should decide whether to crash or to error.
    pub fn compose<T>(
        &self,
        captured: impl IntoIterator<Item = T>,
        provided: impl IntoIterator<Item = T>,
    ) -> Option<Vec<T>> {
        let mut captured = captured.into_iter();
        let mut provided = provided.into_iter();
        let mut result = vec![];
        let mut mask = self.0;
        while mask != 0 {
            if mask & 0x1 != 0 {
                result.push(captured.next()?)
            } else {
                result.push(provided.next()?)
            }
            mask >>= 1;
        }
        if captured.next().is_some() {
            // Not all captured arguments consumed
            return None;
        }
        result.extend(provided);
        Some(result)
    }

    /// Return the max index of captured arguments
    pub fn max_captured(&self) -> usize {
        let mut i = 0;
        let mut mask = self.0;
        while mask != 0 {
            mask >>= 1;
            i += 1
        }
        i
    }

    pub fn merge_placeholder_strings(
        &self,
        arity: usize,
        captured: Vec<String>,
    ) -> Option<Vec<String>> {
        let provided = (0..arity - captured.len())
            .map(|_| "_".to_string())
            .collect::<Vec<_>>();
        self.compose(captured, provided)
    }
}
