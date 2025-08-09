// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{assert_success, tests::common, MoveHarness};
use aptos_types::{
    account_address::AccountAddress,
    transaction::{ExecutionStatus, TransactionStatus},
};
use move_core_types::vm_status::StatusCode;
use test_case::test_case;

fn assert_dependency_limit_reached(status: TransactionStatus) {
    assert!(matches!(
        status,
        TransactionStatus::Keep(ExecutionStatus::MiscellaneousError(Some(
            StatusCode::DEPENDENCY_LIMIT_REACHED
        )))
    ));
}

#[test_case(true, true)]
#[test_case(true, false)]
#[test_case(false, true)]
#[test_case(false, false)]
fn exceeding_max_num_dependencies_on_publish(
    enable_lazy_loading: bool,
    change_max_num_dependencies: bool,
) {
    let mut h = MoveHarness::new_with_lazy_loading(enable_lazy_loading);
    let acc = h.new_account_at(AccountAddress::from_hex_literal("0xcafe").unwrap());

    if change_max_num_dependencies {
        h.modify_gas_schedule(|gas_params| {
            gas_params.vm.txn.max_num_dependencies = 2.into();
        });
    } else {
        // Enough to cover for 2 modules combined: p1 and p2 or p2 and p3.
        h.modify_gas_schedule(|gas_params| {
            gas_params.vm.txn.max_total_dependency_size = 320.into();
        });
    }

    assert_success!(
        h.publish_package_cache_building(&acc, &common::test_dir_path("dependencies.data/p1"))
    );
    assert_success!(
        h.publish_package_cache_building(&acc, &common::test_dir_path("dependencies.data/p2"))
    );

    // Since lazy loading only checks immediate dependencies, and p3 depends on p2 only, publishing
    // should succeed.
    let res =
        h.publish_package_cache_building(&acc, &common::test_dir_path("dependencies.data/p3"));
    if enable_lazy_loading {
        assert_success!(res);
    } else {
        assert_dependency_limit_reached(res);

        // Publishing should succeed if we increase the limit.
        if change_max_num_dependencies {
            h.modify_gas_schedule(|gas_params| {
                gas_params.vm.txn.max_num_dependencies = 3.into();
            });
        } else {
            h.modify_gas_schedule(|gas_params| {
                gas_params.vm.txn.max_total_dependency_size = 1000000.into();
            });
        }

        assert_success!(
            h.publish_package_cache_building(&acc, &common::test_dir_path("dependencies.data/p3"))
        );
    }

    // Should be able to use module in both cases.
    assert_success!(h.run_entry_function(
        &acc,
        str::parse("0xcafe::m3::noop").unwrap(),
        vec![],
        vec![],
    ));
}

#[test_case(true, true)]
#[test_case(true, false)]
#[test_case(false, true)]
#[test_case(false, false)]
fn exceeding_max_num_dependencies(enable_lazy_loading: bool, change_max_num_dependencies: bool) {
    let mut h = MoveHarness::new_with_lazy_loading(enable_lazy_loading);
    let acc = h.new_account_at(AccountAddress::from_hex_literal("0xcafe").unwrap());

    assert_success!(
        h.publish_package_cache_building(&acc, &common::test_dir_path("dependencies.data/p1"))
    );
    assert_success!(
        h.publish_package_cache_building(&acc, &common::test_dir_path("dependencies.data/p2"))
    );
    assert_success!(
        h.publish_package_cache_building(&acc, &common::test_dir_path("dependencies.data/p3"))
    );

    assert_success!(h.run_entry_function(
        &acc,
        str::parse("0xcafe::m3::noop").unwrap(),
        vec![],
        vec![]
    ));
    assert_success!(h.run_entry_function(
        &acc,
        str::parse("0xcafe::m3::load_m2_m1").unwrap(),
        vec![],
        vec![]
    ));

    if change_max_num_dependencies {
        h.modify_gas_schedule(|gas_params| {
            gas_params.vm.txn.max_num_dependencies = 2.into();
        });
    } else {
        h.modify_gas_schedule(|gas_params| {
            gas_params.vm.txn.max_total_dependency_size = 260.into();
        });
    }

    // Here function does not load any modules, so with lazy loading it should run successfully.
    let res = h.run_entry_function(
        &acc,
        str::parse("0xcafe::m3::noop").unwrap(),
        vec![],
        vec![],
    );
    if enable_lazy_loading {
        assert_success!(res);
    } else {
        assert_dependency_limit_reached(res);
    }

    // For both lazy and eager loading, we load 3 modules here and so it must fail.
    let res = h.run_entry_function(
        &acc,
        str::parse("0xcafe::m3::load_m2_m1").unwrap(),
        vec![],
        vec![],
    );
    assert_dependency_limit_reached(res);
}
