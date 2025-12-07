// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

pub const LEDGER_DB_NAME: &str = "ledger_db";
pub const STATE_MERKLE_DB_NAME: &str = "state_merkle_db";

// TODO: Either implement an iteration API to allow a very old client to loop through a long history
// or guarantee that there is always a recent enough waypoint and client knows to boot from there.
pub(crate) const MAX_NUM_EPOCH_ENDING_LEDGER_INFO: usize = 100;
