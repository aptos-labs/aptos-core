pub(crate) mod binary;
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
pub(crate) const MOVE_ZERO: &str = "0";
pub(crate) const MOVE_ZERO_U8: &str = "0u8";
pub(crate) const MOVE_ZERO_U16: &str = "0u16";
pub(crate) const MOVE_ZERO_U32: &str = "0u32";
pub(crate) const MOVE_ZERO_U64: &str = "0u64";
pub(crate) const MOVE_ZERO_U128: &str = "0u128";
pub(crate) const MOVE_ZERO_U256: &str = "0u256";
pub(crate) const MOVE_MAX_U8: &str = "255u8";
pub(crate) const MOVE_MAX_U16: &str = "65535u16";
pub(crate) const MOVE_MAX_U32: &str = "4294967295u32";
pub(crate) const MOVE_MAX_U64: &str = "18446744073709551615u64";
pub(crate) const MOVE_MAX_U128: &str = "340282366920938463463374607431768211455u128";
pub(crate) const MOVE_MAX_U256: &str =
    "115792089237316195423570985008687907853269984665640564039457584007913129639935u256";
pub(crate) const MOVE_MAX_INFERRED_NUM: &str =
    "115792089237316195423570985008687907853269984665640564039457584007913129639935";
pub(crate) const MOVE_ADDR_ZERO: &str = "0x0";
pub(crate) const MOVE_ADDR_MAX: &str =
    "0xFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF";
