use move_model::ast::Exp;
use move_model::model::Loc;

pub(crate) mod binary;
pub(crate) mod binary_swap;

pub(crate) mod break_continue;
pub(crate) mod delete_stmt;
pub(crate) mod ifelse;
pub(crate) mod literal;
pub(crate) mod unary;

// Section with Move constants.
pub(crate) const MOVE_EMPTY_STMT: &str = "{}";
pub(crate) const MOVE_CONTINUE: &str = "continue";
pub(crate) const MOVE_BREAK: &str = "break";
pub(crate) const MOVE_TRUE: &str = "true";
pub(crate) const MOVE_FALSE: &str = "false";
pub(crate) const MOVE_ZERO_U256: &str = "0u256";
pub(crate) const MOVE_MAX_U256: &str =
    "115792089237316195423570985008687907853269984665640564039457584007913129639935u256";
pub(crate) const MOVE_MAX_INFERRED_NUM: &str =
    "115792089237316195423570985008687907853269984665640564039457584007913129639935";
pub(crate) const MOVE_ADDR_ZERO: &str = "0x0";
pub(crate) const MOVE_ADDR_MAX: &str =
    "0xFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF";

#[derive(Debug, Clone)]
pub struct ExpLoc {
    pub exp: Exp,
    pub loc: Loc,
}

impl ExpLoc {
    /// Creates a new expression with location.
    #[cfg(test)]
    pub fn new(exp: Exp, loc: Loc) -> Self {
        Self { exp, loc }
    }
}
