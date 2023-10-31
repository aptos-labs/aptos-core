// Copyright © Aptos Foundation

use move_binary_format::CompiledModule;
use move_core_types::identifier::Identifier;
use move_vm_runtime::move_vm::MoveVM;
use move_vm_test_utils::BlankStorage;
use move_vm_types::gas::UnmeteredGasMeter;

const BLOB: &[u8] = include_bytes!("module_bomb.mv");

#[test]
fn test_module_bomb() {
    let m = CompiledModule::deserialize(BLOB).unwrap();

    let mut vms = vec![];

    for _i in 0..2 {
        let vm = MoveVM::new(vec![]).unwrap();
        let storage = BlankStorage;
        let mut sess = vm.new_session(&storage);
        sess.publish_module(
            BLOB.to_vec(),
            *m.self_id().address(),
            &mut UnmeteredGasMeter,
        )
        .unwrap();

        sess.load_function(&m.self_id(), &Identifier::new("f1").unwrap(), &[])
            .unwrap();
        vms.push(vm);
    }

    let stats = memory_stats::memory_stats().unwrap();
    println!("Physical: {}", stats.physical_mem);
    println!("Virtual: {}", stats.virtual_mem);
}
