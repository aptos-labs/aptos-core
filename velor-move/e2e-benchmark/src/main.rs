// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use velor_language_e2e_tests::{
    account::Account,
    executor::{ExecFuncTimerDynamicArgs, FakeExecutor, GasMeterType, Measurement},
};
use velor_transaction_generator_lib::{
    entry_point_trait::{AutomaticArgs, EntryPointTrait, MultiSigConfig},
    publishing::publish_util::{Package, PackageHandler},
};
use velor_transaction_workloads_lib::{EntryPoints, LoopType, MapType, OrderBookState};
use velor_types::{
    account_address::AccountAddress, chain_id::ChainId, transaction::TransactionPayload,
};
use clap::Parser;
use rand::{rngs::StdRng, SeedableRng};
use serde_json::json;
use std::{collections::HashMap, fs, process::exit};

// bump after a bigger test or perf change, so you can easily distinguish runs
// that are on top of this commit
const CODE_PERF_VERSION: &str = "v1";

pub fn execute_txn(
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
                _ => todo!(),
            },
        },
        GasMeterType::RegularGasMeter,
    )
}

const ALLOWED_REGRESSION: f64 = 0.15;
const ALLOWED_IMPROVEMENT: f64 = 0.15;
const ABSOLUTE_BUFFER_US: f64 = 2.0;

struct CalibrationInfo {
    // count: usize,
    expected_time_micros: f64,
    min_ratio: f64,
    max_ratio: f64,
}

fn get_parsed_calibration_values() -> HashMap<String, CalibrationInfo> {
    let calibration_values =
        fs::read_to_string("velor-move/e2e-benchmark/data/calibration_values.tsv")
            .expect("Unable to read file");
    calibration_values
        .trim()
        .split('\n')
        .map(|line| {
            let parts = line.split('\t').collect::<Vec<_>>();
            (parts[0].to_string(), CalibrationInfo {
                // count: parts[1].parse().unwrap(),
                expected_time_micros: parts[parts.len() - 1].parse().expect(line),
                min_ratio: parts[2].parse().expect(line),
                max_ratio: parts[3].parse().expect(line),
            })
        })
        .collect()
}

#[derive(Parser, Debug)]
struct Args {
    #[clap(long, default_value = "false")]
    pub only_landblocking: bool,
}

// making constants to allow for easier change of type and addition of othe options
const LANDBLOCKING_AND_CONTINUOUS: bool = true;
const ONLY_CONTINUOUS: bool = false;

fn main() {
    let args = Args::parse();
    let executor = FakeExecutor::from_head_genesis();
    let mut executor = executor.set_not_parallel();

    let calibration_values = get_parsed_calibration_values();

    let entry_points = vec![
        // too fast for the timer
        // (, EntryPoints::Nop),
        // (, EntryPoints::BytesMakeOrChange {
        //     data_length: Some(32),
        // }),
        // (, EntryPoints::IncGlobal),
        (LANDBLOCKING_AND_CONTINUOUS, EntryPoints::Loop {
            loop_count: Some(100000),
            loop_type: LoopType::NoOp,
        }),
        (LANDBLOCKING_AND_CONTINUOUS, EntryPoints::Loop {
            loop_count: Some(10000),
            loop_type: LoopType::Arithmetic,
        }),
        // This is a cheap bcs (serializing vec<u8>), so not representative of what BCS native call should cost.
        // (, EntryPoints::Loop { loop_count: Some(1000), loop_type: LoopType::BcsToBytes { len: 1024 }}),
        (LANDBLOCKING_AND_CONTINUOUS, EntryPoints::CreateObjects {
            num_objects: 10,
            object_payload_size: 0,
        }),
        (LANDBLOCKING_AND_CONTINUOUS, EntryPoints::CreateObjects {
            num_objects: 10,
            object_payload_size: 10 * 1024,
        }),
        (ONLY_CONTINUOUS, EntryPoints::CreateObjects {
            num_objects: 100,
            object_payload_size: 0,
        }),
        (ONLY_CONTINUOUS, EntryPoints::CreateObjects {
            num_objects: 100,
            object_payload_size: 10 * 1024,
        }),
        (
            LANDBLOCKING_AND_CONTINUOUS,
            EntryPoints::InitializeVectorPicture { length: 128 },
        ),
        (LANDBLOCKING_AND_CONTINUOUS, EntryPoints::VectorPicture {
            length: 128,
        }),
        (
            LANDBLOCKING_AND_CONTINUOUS,
            EntryPoints::VectorPictureRead { length: 128 },
        ),
        (ONLY_CONTINUOUS, EntryPoints::InitializeVectorPicture {
            length: 30 * 1024,
        }),
        (ONLY_CONTINUOUS, EntryPoints::VectorPicture {
            length: 30 * 1024,
        }),
        (ONLY_CONTINUOUS, EntryPoints::VectorPictureRead {
            length: 30 * 1024,
        }),
        (
            LANDBLOCKING_AND_CONTINUOUS,
            EntryPoints::SmartTablePicture {
                length: 30 * 1024,
                num_points_per_txn: 200,
            },
        ),
        (ONLY_CONTINUOUS, EntryPoints::SmartTablePicture {
            length: 1024 * 1024,
            num_points_per_txn: 300,
        }),
        (
            LANDBLOCKING_AND_CONTINUOUS,
            EntryPoints::ResourceGroupsSenderWriteTag {
                string_length: 1024,
            },
        ),
        (
            LANDBLOCKING_AND_CONTINUOUS,
            EntryPoints::ResourceGroupsSenderMultiChange {
                string_length: 1024,
            },
        ),
        (
            LANDBLOCKING_AND_CONTINUOUS,
            EntryPoints::TokenV1MintAndTransferFT,
        ),
        (
            LANDBLOCKING_AND_CONTINUOUS,
            EntryPoints::TokenV1MintAndTransferNFTSequential,
        ),
        (
            LANDBLOCKING_AND_CONTINUOUS,
            EntryPoints::TokenV2AmbassadorMint { numbered: true },
        ),
        (ONLY_CONTINUOUS, EntryPoints::LiquidityPoolSwap {
            is_stable: true,
        }),
        (
            LANDBLOCKING_AND_CONTINUOUS,
            EntryPoints::LiquidityPoolSwap { is_stable: false },
        ),
        (LANDBLOCKING_AND_CONTINUOUS, EntryPoints::CoinInitAndMint),
        (LANDBLOCKING_AND_CONTINUOUS, EntryPoints::FungibleAssetMint),
        (
            LANDBLOCKING_AND_CONTINUOUS,
            EntryPoints::IncGlobalMilestoneAggV2 { milestone_every: 1 },
        ),
        (ONLY_CONTINUOUS, EntryPoints::IncGlobalMilestoneAggV2 {
            milestone_every: 2,
        }),
        (LANDBLOCKING_AND_CONTINUOUS, EntryPoints::EmitEvents {
            count: 1000,
        }),
        (
            LANDBLOCKING_AND_CONTINUOUS,
            EntryPoints::APTTransferWithPermissionedSigner,
        ),
        (
            LANDBLOCKING_AND_CONTINUOUS,
            EntryPoints::APTTransferWithMasterSigner,
        ),
        // long vectors with small elements
        (LANDBLOCKING_AND_CONTINUOUS, EntryPoints::VectorTrimAppend {
            // baseline, only vector creation
            vec_len: 3000,
            element_len: 1,
            index: 0,
            repeats: 0,
        }),
        (LANDBLOCKING_AND_CONTINUOUS, EntryPoints::VectorTrimAppend {
            vec_len: 3000,
            element_len: 1,
            index: 100,
            repeats: 1000,
        }),
        (ONLY_CONTINUOUS, EntryPoints::VectorTrimAppend {
            vec_len: 3000,
            element_len: 1,
            index: 2990,
            repeats: 1000,
        }),
        (
            LANDBLOCKING_AND_CONTINUOUS,
            EntryPoints::VectorRemoveInsert {
                vec_len: 3000,
                element_len: 1,
                index: 100,
                repeats: 1000,
            },
        ),
        (ONLY_CONTINUOUS, EntryPoints::VectorRemoveInsert {
            vec_len: 3000,
            element_len: 1,
            index: 2998,
            repeats: 1000,
        }),
        (LANDBLOCKING_AND_CONTINUOUS, EntryPoints::VectorRangeMove {
            vec_len: 3000,
            element_len: 1,
            index: 1000,
            move_len: 500,
            repeats: 1000,
        }),
        // vectors with large elements
        (ONLY_CONTINUOUS, EntryPoints::VectorTrimAppend {
            // baseline, only vector creation
            vec_len: 100,
            element_len: 100,
            index: 0,
            repeats: 0,
        }),
        (LANDBLOCKING_AND_CONTINUOUS, EntryPoints::VectorTrimAppend {
            vec_len: 100,
            element_len: 100,
            index: 10,
            repeats: 1000,
        }),
        (LANDBLOCKING_AND_CONTINUOUS, EntryPoints::VectorRangeMove {
            vec_len: 100,
            element_len: 100,
            index: 50,
            move_len: 10,
            repeats: 1000,
        }),
        (LANDBLOCKING_AND_CONTINUOUS, EntryPoints::MapInsertRemove {
            len: 100,
            repeats: 100,
            map_type: MapType::OrderedMap,
        }),
        (LANDBLOCKING_AND_CONTINUOUS, EntryPoints::MapInsertRemove {
            len: 100,
            repeats: 100,
            map_type: MapType::SimpleMap,
        }),
        (LANDBLOCKING_AND_CONTINUOUS, EntryPoints::MapInsertRemove {
            len: 100,
            repeats: 100,
            map_type: MapType::BigOrderedMap {
                inner_max_degree: 4,
                leaf_max_degree: 4,
            },
        }),
        (ONLY_CONTINUOUS, EntryPoints::MapInsertRemove {
            len: 100,
            repeats: 100,
            map_type: MapType::BigOrderedMap {
                inner_max_degree: 1024,
                leaf_max_degree: 1024,
            },
        }),
        (ONLY_CONTINUOUS, EntryPoints::MapInsertRemove {
            len: 1000,
            repeats: 100,
            map_type: MapType::OrderedMap,
        }),
        (LANDBLOCKING_AND_CONTINUOUS, EntryPoints::OrderBook {
            state: OrderBookState::new(),
            num_markets: 1,
            overlap_ratio: 0.0, // Since we run a single txn, no matches will happen irrespectively
            buy_frequency: 0.5,
            max_sell_size: 1,
            max_buy_size: 1,
        }),
    ];

    let mut failures = Vec::new();
    let mut json_lines = Vec::new();

    println!(
        "{:>13} {:>13} {:>13}{:>13} {:>13} {:>13}  entry point",
        "walltime(us)", "expected(us)", "dif(- is impr)", "gas/s", "exe gas", "io gas",
    );

    for (index, (flow, entry_point)) in entry_points.into_iter().enumerate() {
        if args.only_landblocking && (flow == ONLY_CONTINUOUS) {
            continue;
        }
        let entry_point_name = format!("{:?}", entry_point);
        let cur_calibration = calibration_values
            .get(&entry_point_name)
            .expect(&entry_point_name);
        let expected_time_micros = cur_calibration.expected_time_micros;
        let publisher = executor.new_account_at(AccountAddress::random());

        let mut package_handler =
            PackageHandler::new(entry_point.pre_built_packages(), entry_point.package_name());
        let mut rng = StdRng::seed_from_u64(14);
        let package = package_handler.pick_package(&mut rng, *publisher.address());
        for payload in package.publish_transaction_payload(&ChainId::test()) {
            execute_txn(&mut executor, &publisher, 0, payload);
        }
        if let Some(init_entry_point) = entry_point.initialize_entry_point() {
            execute_txn(
                &mut executor,
                &publisher,
                1,
                init_entry_point.create_payload(
                    &package,
                    init_entry_point.module_name(),
                    Some(&mut rng),
                    Some(publisher.address()),
                ),
            );
        }

        let measurement = execute_and_time_entry_point(
            &entry_point,
            &package,
            publisher.address(),
            &mut executor,
            if expected_time_micros > 10000.0 {
                6
            } else if expected_time_micros > 1000.0 {
                10
            } else {
                100
            },
        );
        let elapsed_micros = measurement.elapsed_micros_f64();
        let diff = (elapsed_micros - expected_time_micros) / expected_time_micros * 100.0;
        let execution_gas_units = measurement.execution_gas_units();
        let io_gas_units = measurement.io_gas_units();
        let gps = (execution_gas_units + io_gas_units) / measurement.elapsed_secs_f64();
        println!(
            "{:13.1} {:13.1} {:12.1}% {:13.0} {:13.2} {:13.2}  {:?}",
            elapsed_micros,
            expected_time_micros,
            diff,
            gps,
            execution_gas_units,
            io_gas_units,
            entry_point
        );

        let max_regression = f64::max(
            expected_time_micros * (1.0 + ALLOWED_REGRESSION) + ABSOLUTE_BUFFER_US,
            expected_time_micros * cur_calibration.max_ratio,
        );
        let max_improvement = f64::min(
            expected_time_micros * (1.0 - ALLOWED_IMPROVEMENT) - ABSOLUTE_BUFFER_US,
            expected_time_micros * cur_calibration.min_ratio,
        );

        json_lines.push(json!({
            "grep": "grep_json_velor_move_vm_perf",
            "transaction_type": entry_point_name,
            "wall_time_us": elapsed_micros,
            "gas_units_per_second": gps,
            "execution_gas_units": execution_gas_units,
            "io_gas_units": io_gas_units,
            "expected_wall_time_us": expected_time_micros,
            "expected_max_wall_time_us": max_regression,
            "expected_min_wall_time_us": max_improvement,
            "code_perf_version": CODE_PERF_VERSION,
            "test_index": index,
            "flow": if args.only_landblocking { "LAND_BLOCKING" } else { "CONTINUOUS" },
        }));

        if elapsed_micros > max_regression {
            failures.push(format!(
                "Performance regression detected: {:.1}us, expected: {:.1}us, limit: {:.1}us, diff: {}%, for {:?}",
                elapsed_micros, expected_time_micros, max_regression, diff, entry_point
            ));
        } else if elapsed_micros < max_improvement {
            failures.push(format!(
                "Performance improvement detected: {:.1}us, expected {:.1}us, limit {:.1}us, diff: {}%, for {:?}. You need to adjust expected time!",
                elapsed_micros, expected_time_micros, max_improvement, diff, entry_point
            ));
        }
    }

    for line in json_lines {
        println!("{}", serde_json::to_string(&line).unwrap());
    }

    for failure in &failures {
        println!("{}", failure);
    }
    if !failures.is_empty() {
        println!("Failing, there were perf improvements or regressions.");
        exit(1);
    }

    // Assert there were no error log lines in the run.
    assert_eq!(
        0,
        velor_logger::ERROR_LOG_COUNT.get(),
        "Error logs were found in the run."
    );
}
