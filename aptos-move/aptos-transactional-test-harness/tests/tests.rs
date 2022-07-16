// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use aptos_transactional_test_harness::run_aptos_test;

datatest_stable::harness!(run_aptos_test, "tests", r".*\.(mvir|move)$");
