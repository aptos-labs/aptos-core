// Copyright (c) The Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use diem_transactional_test_harness::run_dpn_test;

datatest_stable::harness!(run_dpn_test, "tests", r".*\.(mvir|move)$");
