// Copyright © Aptos Foundation

#[derive(Debug, Clone)]
pub enum Expression {
    Add {
        left: Box<Expression>,
        right: Box<Expression>,
    },
    Mul {
        left: Box<Expression>,
        right: Box<Expression>,
    },
    GasParam {
        name: String,
    },
    GasValue {
        value: u64,
    },
}
