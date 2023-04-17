// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use aptos_transactional_test_harness::run_aptos_test;

//////// 0L ////////
// 0L: libra and aptos-core repos must be in the same dir
// datatest_stable::harness!(run_aptos_test, "tests", r".*\.(mvir|move)$");
datatest_stable::harness!(run_aptos_test, "../../../libra/transactional_tests/tests", r".*\.(mvir|move)$");
