// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use aptos_language_e2e_tests::{
    account::Account,
    executor::{ExecFuncTimerDynamicArgs, FakeExecutor, GasMeterType, Measurement},
};
use aptos_transaction_generator_lib::{
    entry_point_trait::{AutomaticArgs, EntryPointTrait, MultiSigConfig},
    publishing::publish_util::{Package, PackageHandler},
};
use aptos_transaction_workloads_lib::{EntryPoints, MoveVmMicroBenchmark};
use aptos_types::{account_address::AccountAddress, transaction::TransactionPayload};
use rand::{rngs::StdRng, SeedableRng};

fn execute_txn(
    executor: &mut FakeExecutor,
    account: &Account,
    sequence_number: u64,
    payload: TransactionPayload,
) {
    let sign_tx = account
        .transaction()
        .sequence_number(sequence_number)
        .max_gas_amount(2_000_000)
        .gas_unit_price(200)
        .payload(payload)
        .sign();
    let txn_output = executor.execute_transaction(sign_tx);
    executor.apply_write_set(txn_output.write_set());
    assert!(
        txn_output.status().status().unwrap().is_success(),
        "txn failed with {:?}",
        txn_output.status()
    );
}

fn execute_and_time_entry_point(
    entry_point: &EntryPoints,
    package: &Package,
    publisher_address: &AccountAddress,
    executor: &mut FakeExecutor,
    iterations: u64,
) -> Measurement {
    let mut rng = StdRng::seed_from_u64(14);
    let entry_fun = entry_point
        .create_payload(
            package,
            entry_point.module_name(),
            Some(&mut rng),
            Some(publisher_address),
        )
        .into_entry_function();

    executor.exec_func_record_running_time(
        entry_fun.module(),
        entry_fun.function().as_str(),
        entry_fun.ty_args().to_vec(),
        entry_fun.args().to_vec(),
        iterations,
        match entry_point.automatic_args() {
            AutomaticArgs::None => ExecFuncTimerDynamicArgs::NoArgs,
            AutomaticArgs::Signer => ExecFuncTimerDynamicArgs::DistinctSigners,
            AutomaticArgs::SignerAndMultiSig => match entry_point.multi_sig_additional_num() {
                MultiSigConfig::Publisher => {
                    ExecFuncTimerDynamicArgs::DistinctSignersAndFixed(vec![*publisher_address])
                },
                _ => unimplemented!("multi-sig variant not supported here"),
            },
        },
        GasMeterType::RegularGasMeter,
    )
}

fn run() {
    let executor = FakeExecutor::from_head_genesis();
    let mut executor = executor.set_not_parallel();

    let workloads = [
        EntryPoints::MoveVmMicroBenchmark(MoveVmMicroBenchmark::Locals),
        EntryPoints::MoveVmMicroBenchmark(MoveVmMicroBenchmark::LocalsGeneric),
    ];

    println!(
        "{:>13} {:>13} {:>13}  entry point",
        "walltime(us)", "exe gas", "io gas"
    );

    for ep in workloads.iter() {
        let publisher = executor.new_account_at(AccountAddress::random());
        let mut package_handler = PackageHandler::new(ep.pre_built_packages(), ep.package_name());
        let mut rng = StdRng::seed_from_u64(14);
        let package = package_handler.pick_package(&mut rng, *publisher.address());
        for payload in package.publish_transaction_payload(&aptos_types::chain_id::ChainId::test())
        {
            execute_txn(&mut executor, &publisher, 0, payload);
        }

        let package = {
            let mut handler = PackageHandler::new(ep.pre_built_packages(), ep.package_name());
            let mut rng = StdRng::seed_from_u64(14);
            handler.pick_package(&mut rng, *publisher.address())
        };
        let measurement =
            execute_and_time_entry_point(ep, &package, publisher.address(), &mut executor, 50);
        let elapsed = measurement.elapsed_micros_f64();
        let execution_gas_units = measurement.execution_gas_units();
        let io_gas_units = measurement.io_gas_units();
        println!(
            "{:13.1} {:13.2} {:13.2}  {:?}",
            elapsed, execution_gas_units, io_gas_units, ep
        );
    }
}

fn main() {
    run();
}
