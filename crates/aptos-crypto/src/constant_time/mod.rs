// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! This module provides implementations of "dudect" statistical tests to check some of our code
//! is constant-time (e.g., like scalar multiplication).

/// Module for testing that blstrs scalar multiplication is constant-time
pub mod blstrs_scalar_mul;
/// Module for testing that zkcrypto scalar multiplication is constant-time
pub mod zkcrypto_scalar_mul;
