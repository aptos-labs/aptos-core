// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Options controlling how a VM value renders as text.
//!
//! # Divergence from the V1 formatter
//!
//! V1 builds no named layout for an enum, so it prints an enum as `#0{ 7 }`:
//! the tag stands in for the variant, and the whole subtree below loses its
//! names too. A struct nested in a variant is `{ 1, true }`, a `String` is its
//! raw hex, and an `Option` is `#1{ 5 }`. Mono prints `E::Variant { r: 7 }` and
//! keeps the subtree decorated. This is deliberate and not configurable; the
//! differential suite pins both renderings with `CHECK-V1` / `CHECK-V2`.

/// Controls the textual rendering of a VM value.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FormatOptions {
    /// Values stay on one line; otherwise aggregates break across lines with a
    /// two-space indent per nesting level.
    pub single_line: bool,
    /// Struct headers print `0x1::m::S<u64>` rather than `S`.
    pub fully_qualified_nominals: bool,
    /// Addresses print as 64 hex digits rather than a trimmed `0x` literal.
    pub canonical_addresses: bool,
    /// Integers carry a width suffix, e.g. `1u64`.
    pub int_suffixes: bool,

    /// `vector<u8>` prints as `0x<hex>` rather than a list.
    pub vec_u8_as_hex: bool,
    /// `0x1::string::String` prints as a quoted, escaped literal.
    pub string_literals: bool,

    /// Aggregates nested deeper than this render as ` .. `.
    pub max_depth: usize,
    /// Elements past this index render as `..`.
    pub max_len: usize,
}

impl FormatOptions {
    /// `0x1::string_utils::to_string_with_canonical_addresses`.
    pub const CANONICAL_ADDRESSES: Self = Self {
        canonical_addresses: true,
        ..Self::TO_STRING
    };
    /// `0x1::string_utils::debug_string`, also the rendering behind
    /// `0x1::debug::print`.
    pub const DEBUG_STRING: Self = Self {
        fully_qualified_nominals: true,
        ..Self::MONO_MOVE
    };
    /// One `{}` substitution of `0x1::string_utils::format1`..`format4`.
    pub const LIST_ELEMENT: Self = Self {
        single_line: true,
        fully_qualified_nominals: true,
        ..Self::MONO_MOVE
    };
    /// Multi-line, unqualified, unabridged.
    // TODO(metering): replace the two limits with finite bounds, large enough
    // that nothing in practice hits them, so every walk terminates.
    pub const MONO_MOVE: Self = Self {
        single_line: false,
        fully_qualified_nominals: false,
        canonical_addresses: false,
        int_suffixes: false,
        vec_u8_as_hex: true,
        string_literals: true,
        max_depth: usize::MAX,
        max_len: usize::MAX,
    };
    /// `0x1::string_utils::to_string`.
    pub const TO_STRING: Self = Self {
        single_line: true,
        ..Self::MONO_MOVE
    };
}

impl Default for FormatOptions {
    fn default() -> Self {
        Self::MONO_MOVE
    }
}
