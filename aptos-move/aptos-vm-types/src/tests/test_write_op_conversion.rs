use crate::change_set::VMChangeSet;
use aptos_aggregator::{
    delta_change_set::{delta_add, delta_sub, serialize},
    AggregatorStore,
};
use aptos_state_view::TStateView;
use aptos_types::{
    state_store::{
        state_key::StateKey, state_storage_usage::StateStorageUsage, state_value::StateValue,
    },
    write_set::WriteOp,
};
use claims::{assert_matches, assert_ok_eq};
use move_core_types::vm_status::{StatusCode, VMStatus};
use once_cell::sync::Lazy;

static KEY: Lazy<StateKey> = Lazy::new(|| StateKey::raw(String::from("test-key").into_bytes()));

struct BadStorage;

impl TStateView for BadStorage {
    type Key = StateKey;

    fn get_state_value(&self, _state_key: &Self::Key) -> anyhow::Result<Option<StateValue>> {
        Err(anyhow::Error::new(VMStatus::error(
            StatusCode::STORAGE_ERROR,
            Some("Error message from BadStorage.".to_string()),
        )))
    }

    fn get_usage(&self) -> anyhow::Result<StateStorageUsage> {
        unreachable!()
    }
}

#[test]
fn test_failed_write_op_conversion_because_of_storage_error() {
    let state_view = BadStorage;
    let delta_op = delta_add(10, 1000);
    assert_matches!(
        VMChangeSet::try_into_write_op(delta_op, &state_view, &KEY),
        Err(VMStatus::Error {
            status_code: StatusCode::STORAGE_ERROR,
            message: Some(_),
            sub_status: None
        })
    );
}

#[test]
fn test_failed_write_op_conversion_because_of_empty_storage() {
    let state_view = AggregatorStore::default();
    let delta_op = delta_add(10, 1000);
    assert_matches!(
        VMChangeSet::try_into_write_op(delta_op, &state_view, &KEY),
        Err(VMStatus::Error {
            status_code: StatusCode::STORAGE_ERROR,
            message: Some(_),
            sub_status: None
        })
    );
}

#[test]
fn test_successful_write_op_conversion() {
    let mut state_view = AggregatorStore::default();
    state_view.set_from_state_key(KEY.clone(), 100);

    // Both addition and subtraction should succeed!
    let add_op = delta_add(100, 200);
    let sub_op = delta_sub(100, 200);

    let add_result = VMChangeSet::try_into_write_op(add_op, &state_view, &KEY);
    assert_ok_eq!(add_result, WriteOp::Modification(serialize(&200)));

    let sub_result = VMChangeSet::try_into_write_op(sub_op, &state_view, &KEY);
    assert_ok_eq!(sub_result, WriteOp::Modification(serialize(&0)));
}

#[test]
fn test_unsuccessful_write_op_conversion() {
    let mut state_view = AggregatorStore::default();
    state_view.set_from_state_key(KEY.clone(), 100);

    // Both addition and subtraction should fail!
    let add_op = delta_add(15, 100);
    let sub_op = delta_sub(101, 1000);

    const EADD_OVERFLOW: u64 = 0x02_0001;
    assert_matches!(
        VMChangeSet::try_into_write_op(add_op, &state_view, &KEY),
        Err(VMStatus::MoveAbort(_, EADD_OVERFLOW))
    );

    const ESUB_UNDERFLOW: u64 = 0x02_0002;
    assert_matches!(
        VMChangeSet::try_into_write_op(sub_op, &state_view, &KEY),
        Err(VMStatus::MoveAbort(_, ESUB_UNDERFLOW))
    );
}
