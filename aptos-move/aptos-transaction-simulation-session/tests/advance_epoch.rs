// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use anyhow::Result;
use aptos_transaction_simulation::SimulationStateStore;
use aptos_transaction_simulation_session::Session;
use aptos_types::{
    on_chain_config::{ConfigurationResource, CurrentTimeMicroseconds},
    randomness::PerBlockRandomness,
};

#[test]
fn test_advance_epoch() -> Result<()> {
    let temp_dir = tempfile::tempdir()?;
    let mut session = Session::init(temp_dir.path())?;

    let initial: ConfigurationResource = session.state_store().get_on_chain_config()?;

    let result = session.advance_epoch()?;
    assert_eq!(result.old_epoch, initial.epoch());
    assert_eq!(result.new_epoch, initial.epoch() + 1);

    Ok(())
}

#[test]
fn test_advance_epoch_twice() -> Result<()> {
    let temp_dir = tempfile::tempdir()?;
    let mut session = Session::init(temp_dir.path())?;

    let initial: ConfigurationResource = session.state_store().get_on_chain_config()?;

    session.advance_epoch()?;
    let result = session.advance_epoch()?;

    assert_eq!(result.new_epoch, initial.epoch() + 2);

    Ok(())
}

#[test]
fn test_advance_epoch_preserves_randomness_seed() -> Result<()> {
    let temp_dir = tempfile::tempdir()?;
    let mut session = Session::init(temp_dir.path())?;

    session.advance_epoch()?;

    let randomness: PerBlockRandomness = session.state_store().get_on_chain_config()?;
    assert!(randomness.seed.is_some());

    Ok(())
}

#[test]
fn test_advance_epoch_advances_timestamp() -> Result<()> {
    let temp_dir = tempfile::tempdir()?;
    let mut session = Session::init(temp_dir.path())?;

    let before: CurrentTimeMicroseconds = session.state_store().get_on_chain_config()?;
    let result = session.advance_epoch()?;

    assert!(
        result.new_timestamp_usecs > before.microseconds,
        "timestamp should advance past the previous value"
    );

    Ok(())
}

#[test]
fn test_advance_epoch_emits_new_epoch_event() -> Result<()> {
    let temp_dir = tempfile::tempdir()?;
    let mut session = Session::init(temp_dir.path())?;

    session.advance_epoch()?;

    let events_path = temp_dir.path().join("[0] new block").join("events.json");
    let events_json: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&events_path)?)?;
    let events = events_json.as_array().expect("events should be an array");

    // Look for a NewEpoch event (V1 or V2).
    let has_new_epoch_event = events.iter().any(|e| {
        let type_tag = e
            .pointer("/V1/type_tag")
            .or_else(|| e.pointer("/V2/type_tag"));
        matches!(type_tag, Some(serde_json::Value::String(s)) if s.contains("reconfiguration::NewEpoch"))
    });
    assert!(has_new_epoch_event, "should emit a NewEpoch event");

    Ok(())
}
