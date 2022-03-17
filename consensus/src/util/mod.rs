// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

#[cfg(any(test, feature = "fuzzing"))]
pub mod mock_time_service;
pub mod time_service;
