// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use aptos_types::error::PanicError;
use bytes::Bytes;
use move_core_types::value::MoveTypeLayout;

/// Materializes speculative values into their committed form. Implemented by
/// the block executor, which holds the context needed at commit time: the
/// committed delayed field values, the finalized resource group contents and
/// the values of reads that need delayed-field exchange. Passed to the
/// transaction output at materialization so it can patch the data it owns in
/// place, without the executor copying it out and back in.
pub trait Materializer {
    type Key;

    /// Replaces the delayed-field identifiers in the given serialized value
    /// (of the given layout) with the committed values.
    fn replace_identifiers_with_values(
        &self,
        bytes: &[u8],
        layout: &MoveTypeLayout,
    ) -> Result<Bytes, PanicError>;

    /// The serialized bytes of the finalized resource group for a group key
    /// this transaction wrote, or read in a way that needs delayed-field
    /// exchange. Delayed-field identifiers are already replaced with their
    /// committed values.
    fn serialized_group_bytes(&self, key: &Self::Key) -> Result<Bytes, PanicError>;

    /// The bytes of a resource this transaction only read but that needs
    /// delayed-field exchange, with the identifiers already replaced with
    /// their committed values.
    fn exchanged_read_bytes(&self, key: &Self::Key) -> Result<Bytes, PanicError>;
}
