// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! The parts of a transaction's report label the replay tools share.

use aptos_types::{
    block_metadata_ext::BlockMetadataExt,
    transaction::{BlockEpiloguePayload, EntryFunction},
};

/// An entry function's `module::function<type args>`.
pub fn entry_function(entry: &EntryFunction) -> String {
    let mut label = format!(
        "{}::{}",
        entry.module().short_str_lossless(),
        entry.function()
    );
    if !entry.ty_args().is_empty() {
        let ty_args = entry
            .ty_args()
            .iter()
            .map(|t| t.to_canonical_string())
            .collect::<Vec<_>>()
            .join(", ");
        label.push_str(&format!("<{}>", ty_args));
    }
    label
}

/// The block-metadata-ext variant.
pub fn block_metadata_ext(metadata: &BlockMetadataExt) -> &'static str {
    match metadata {
        BlockMetadataExt::V0(_) => "block_metadata_ext_v0",
        BlockMetadataExt::V1(_) => "block_metadata_ext_v1",
        BlockMetadataExt::V2(_) => "block_metadata_ext_v2",
        BlockMetadataExt::V3(_) => "block_metadata_ext_v3",
    }
}

/// The block-epilogue variant.
pub fn block_epilogue(payload: &BlockEpiloguePayload) -> &'static str {
    match payload {
        BlockEpiloguePayload::V0 { .. } => "block_epilogue_v0",
        BlockEpiloguePayload::V1 { .. } => "block_epilogue_v1",
        BlockEpiloguePayload::V2 { .. } => "block_epilogue_v2",
    }
}
