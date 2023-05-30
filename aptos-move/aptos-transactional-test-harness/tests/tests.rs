// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use aptos_transactional_test_harness::run_aptos_test;

//////// 0L ////////
// 0L: TRX_TESTS: the path of ol-fw trans. tests 
// e.g. /opt/libra-v7//transactional-tests/tests/
datatest_stable::harness!(run_aptos_test, "tests", r".*\.(mvir|move)$");
// datatest_stable::harness!(run_aptos_test, env!("TRX_TESTS"), r".*\.(mvir|move)$");
