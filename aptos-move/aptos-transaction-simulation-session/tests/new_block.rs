// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use anyhow::Result;
use aptos_transaction_simulation::SimulationStateStore;
use aptos_transaction_simulation_session::{BlockTimestamp, Session};
use aptos_types::{
    account_address::AccountAddress, account_config::events::new_block::BlockResource,
    on_chain_config::CurrentTimeMicroseconds, randomness::PerBlockRandomness,
};

#[test]
fn test_new_block_default_timestamp() -> Result<()> {
    let temp_dir = tempfile::tempdir()?;
    let mut session = Session::init(temp_dir.path())?;

    let before: CurrentTimeMicroseconds = session.state_store().get_on_chain_config()?;
    let result = session.new_block(BlockTimestamp::Default)?;

    assert_eq!(result.new_timestamp_usecs, before.microseconds + 1);
    assert_eq!(result.old_epoch, result.new_epoch);

    Ok(())
}

#[test]
fn test_new_block_with_absolute_timestamp() -> Result<()> {
    let temp_dir = tempfile::tempdir()?;
    let mut session = Session::init(temp_dir.path())?;

    let new_time = 1_000_000; // 1 second
    let result = session.new_block(BlockTimestamp::Absolute(new_time))?;

    assert_eq!(result.new_timestamp_usecs, new_time);

    let updated: CurrentTimeMicroseconds = session.state_store().get_on_chain_config()?;
    assert_eq!(updated.microseconds, new_time);

    Ok(())
}

#[test]
fn test_new_block_crossing_epoch_boundary() -> Result<()> {
    let temp_dir = tempfile::tempdir()?;
    let mut session = Session::init(temp_dir.path())?;

    let block_resource: BlockResource = session
        .state_store()
        .get_resource(AccountAddress::ONE)?
        .expect("BlockResource should exist");
    let epoch_interval = block_resource.epoch_interval();

    let result = session.new_block(BlockTimestamp::Absolute(epoch_interval))?;
    assert_eq!(result.new_epoch, result.old_epoch + 1);

    Ok(())
}

#[test]
fn test_new_block_not_crossing_epoch_boundary() -> Result<()> {
    let temp_dir = tempfile::tempdir()?;
    let mut session = Session::init(temp_dir.path())?;

    let block_resource: BlockResource = session
        .state_store()
        .get_resource(AccountAddress::ONE)?
        .expect("BlockResource should exist");
    let epoch_interval = block_resource.epoch_interval();

    // 1 microsecond before the boundary.
    let result = session.new_block(BlockTimestamp::Absolute(epoch_interval - 1))?;
    assert_eq!(result.old_epoch, result.new_epoch);

    Ok(())
}

#[test]
fn test_new_block_preserves_randomness_seed() -> Result<()> {
    let temp_dir = tempfile::tempdir()?;
    let mut session = Session::init(temp_dir.path())?;

    session.new_block(BlockTimestamp::Default)?;

    let randomness: PerBlockRandomness = session.state_store().get_on_chain_config()?;
    assert!(
        randomness.seed.is_some(),
        "randomness seed should be re-patched after new_block"
    );

    Ok(())
}

#[test]
fn test_new_block_emits_new_block_event() -> Result<()> {
    let temp_dir = tempfile::tempdir()?;
    let mut session = Session::init(temp_dir.path())?;

    session.new_block(BlockTimestamp::Default)?;

    let events_path = temp_dir.path().join("[0] new block").join("events.json");
    let events_json: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&events_path)?)?;
    let events = events_json.as_array().expect("events should be an array");

    // Look for a NewBlockEvent (V1) or NewBlock (V2).
    let has_new_block_event = events.iter().any(|e| {
        let type_tag = e
            .pointer("/V1/type_tag")
            .or_else(|| e.pointer("/V2/type_tag"));
        matches!(type_tag, Some(serde_json::Value::String(s)) if s.contains("block::NewBlock"))
    });
    assert!(has_new_block_event, "should emit a NewBlock event");

    Ok(())
}
