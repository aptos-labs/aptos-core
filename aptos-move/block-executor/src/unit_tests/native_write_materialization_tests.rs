// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{
    combinatorial_tests::{
        mock_executor::{MockEvent, MockOutput},
        types::{KeyType, MockTransaction, ValueType},
    },
    executor_utilities::{materialize_output, Materializer},
};
use aptos_types::{
    block_executor::value::ValueWithLayout,
    error::{code_invariant_error, PanicError},
    state_store::state_value::StateValueMetadata,
    write_set::{TransactionWrite, WriteOpKind},
};
use aptos_vm_types::resolver::ResourceGroupSize;
use bytes::Bytes;
use claims::assert_ok;
use move_core_types::value::MoveTypeLayout;
use triomphe::Arc as TriompheArc;

type Txn = MockTransaction<KeyType<u32>, MockEvent>;

static LAYOUT: MoveTypeLayout = MoveTypeLayout::U64;

/// Stands in for the delayed field exchange: marks the bytes it was asked to patch.
struct MarkingMaterializer;

impl Materializer<Txn> for MarkingMaterializer {
    fn finalize_group(
        &self,
        _key: &KeyType<u32>,
    ) -> Result<(Vec<(u32, ValueWithLayout<ValueType>)>, ResourceGroupSize), PanicError> {
        Err(code_invariant_error("no groups in this test"))
    }

    fn replace_ids_with_values(
        &self,
        bytes: &Bytes,
        layout: &MoveTypeLayout,
    ) -> Result<Bytes, PanicError> {
        assert!(std::ptr::eq(layout, &LAYOUT));
        let mut patched = b"patched:".to_vec();
        patched.extend_from_slice(bytes);
        Ok(patched.into())
    }

    fn fetch_exchanged_read(
        &self,
        _key: &KeyType<u32>,
    ) -> Result<(TriompheArc<ValueType>, TriompheArc<MoveTypeLayout>), PanicError> {
        Err(code_invariant_error("no exchanged reads in this test"))
    }
}

fn output_with_native_write() -> MockOutput<KeyType<u32>, MockEvent> {
    let mut output = MockOutput::skipped_output(None);
    output.skipped = false;
    output.native_writes_needing_exchange = vec![(
        KeyType(7),
        ValueType::new(
            Some(Bytes::from_static(b"id")),
            StateValueMetadata::none(),
            WriteOpKind::Creation,
        ),
        &LAYOUT,
    )];
    output
}

#[test]
fn native_writes_are_patched_and_incorporated() {
    let (committed, _) = assert_ok!(materialize_output(
        output_with_native_write(),
        &MarkingMaterializer,
        true
    ));
    let patched = committed.patched_native_writes.get().unwrap();
    assert_eq!(patched.len(), 1);
    assert_eq!(patched[0].0, KeyType(7));
    assert_eq!(
        patched[0].1.bytes().unwrap(),
        &Bytes::from_static(b"patched:id")
    );
}

#[test]
fn native_writes_without_delayed_field_optimization_are_rejected() {
    assert!(materialize_output(output_with_native_write(), &MarkingMaterializer, false).is_err());
}

#[test]
fn no_native_writes_materialize_without_delayed_field_optimization() {
    let (committed, _) = assert_ok!(materialize_output(
        MockOutput::<KeyType<u32>, MockEvent>::skipped_output(None),
        &MarkingMaterializer,
        false
    ));
    assert!(committed.patched_native_writes.get().unwrap().is_empty());
}
