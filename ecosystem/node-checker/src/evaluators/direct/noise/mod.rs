// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

mod handshake;

pub use handshake::{HandshakeEvaluator, HandshakeEvaluatorArgs};
use thiserror::Error as ThisError;

pub const NOISE_CATEGORY: &str = "noise";

#[derive(Debug, ThisError)]
pub enum NoiseEvaluatorError {}
