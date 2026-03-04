// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

/// Accept types from input headers
///
/// Determines the output type of each API
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum AcceptType {
    /// Convert and resolve types to JSON
    Json,
    /// Take types with as little conversion as possible from the database
    Bcs,
}
