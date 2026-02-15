// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Model-driven strategy: a single comprehensive prompt lets the model autonomously
//! drive the workflow by calling `verify` and `wp_inference` tools.

pub mod loop_driver;

pub use loop_driver::run_model_driven_loop;
