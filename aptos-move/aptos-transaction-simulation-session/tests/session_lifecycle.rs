// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use anyhow::Result;
use aptos_transaction_simulation::SimulationStateStore;
use aptos_transaction_simulation_session::Session;
use aptos_types::{on_chain_config::CurrentTimeMicroseconds, randomness::PerBlockRandomness};

#[test]
fn test_init_then_load_local() -> Result<()> {
    let temp_dir = tempfile::tempdir()?;

    let _session = Session::init(temp_dir.path())?;
    let loaded = Session::load(temp_dir.path())?;

    // Verify the loaded session can read on-chain state from genesis.
    let timestamp: CurrentTimeMicroseconds = loaded.state_store().get_on_chain_config()?;
    assert_eq!(timestamp.microseconds, 0);

    Ok(())
}

#[test]
fn test_init_local_patches_randomness_seed() -> Result<()> {
    let temp_dir = tempfile::tempdir()?;

    let session = Session::init(temp_dir.path())?;

    let randomness: PerBlockRandomness = session.state_store().get_on_chain_config()?;
    assert!(randomness.seed.is_some(), "randomness seed should be set");
    assert_eq!(randomness.seed.unwrap().len(), 32);

    Ok(())
}

#[test]
fn test_init_fails_on_nonempty_directory() -> Result<()> {
    let temp_dir = tempfile::tempdir()?;
    std::fs::write(temp_dir.path().join("existing_file"), "data")?;

    let result = Session::init(temp_dir.path());
    assert!(result.is_err());

    Ok(())
}
