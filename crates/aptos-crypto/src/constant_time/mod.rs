// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! This module provides implementations of "dudect" statistical tests to check some of our code
//! is constant-time (e.g., like scalar multiplication).

/// Module for testing that blstrs scalar multiplication is constant-time
pub mod blstrs_scalar_mul;
/// Module for testing that zkcrypto scalar multiplication is constant-time
pub mod zkcrypto_scalar_mul;
