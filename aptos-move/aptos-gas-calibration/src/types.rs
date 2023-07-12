// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

/*
 * @notice: Represent abstract algebra into easier format
 */
#[derive(Debug, Clone)]
pub enum Expression {
    //// Represent GasAdd
    Add {
        left: Box<Expression>,
        right: Box<Expression>,
    },
    //// Represent GasMul
    Mul {
        left: Box<Expression>,
        right: Box<Expression>,
    },
    //// Represent GasParam
    GasParam {
        name: String,
    },
    //// Represent GasValue
    GasValue {
        value: u64,
    },
}
