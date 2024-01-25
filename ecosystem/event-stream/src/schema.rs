// Copyright © Aptos Foundation
// @generated automatically by Diesel CLI.

pub mod event_stream {
    diesel::table! {
        event_stream.ledger_infos (chain_id) {
            chain_id -> Int8,
        }
    }
}
