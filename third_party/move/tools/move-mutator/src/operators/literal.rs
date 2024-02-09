use crate::operator::{MutantInfo, MutationOperator};
use crate::operators::{
    MOVE_ADDR_MAX, MOVE_ADDR_ZERO, MOVE_FALSE, MOVE_MAX_INFERRED_NUM, MOVE_MAX_U128, MOVE_MAX_U16,
    MOVE_MAX_U256, MOVE_MAX_U32, MOVE_MAX_U64, MOVE_MAX_U8, MOVE_TRUE, MOVE_ZERO, MOVE_ZERO_U128,
    MOVE_ZERO_U16, MOVE_ZERO_U256, MOVE_ZERO_U32, MOVE_ZERO_U64, MOVE_ZERO_U8,
};
use crate::report::{Mutation, Range};
use move_command_line_common::files::FileHash;
use move_compiler::expansion::ast;
use move_compiler::expansion::ast::Value_;
use std::fmt;
use std::fmt::Debug;

pub const OPERATOR_NAME: &str = "literal_replacement";

/// Literal replacement mutation operator.
/// Replaces literal statements with other ones but withing the same type.
#[derive(Debug, Clone)]
pub struct Literal {
    operation: ast::Value,
}

impl Literal {
    /// Creates a new instance of the literal mutation operator.
    #[must_use]
    pub fn new(operation: ast::Value) -> Self {
        Self { operation }
    }
}

impl MutationOperator for Literal {
    fn apply(&self, source: &str) -> Vec<MutantInfo> {
        let start = self.operation.loc.start() as usize;
        let end = self.operation.loc.end() as usize;
        let cur_op = &source[start..end];

        // Group of literal statements for possible Value types.
        let ops: Vec<String> = match &self.operation.value {
            Value_::Address(_addr) => {
                vec![MOVE_ADDR_ZERO.to_owned(), MOVE_ADDR_MAX.to_owned()]
            },
            Value_::Bool(_bool_val) => {
                vec![MOVE_TRUE.to_owned(), MOVE_FALSE.to_owned()]
            },
            Value_::Bytearray(_bytearray) => {
                vec!["b\"121112\"".to_owned()]
            },
            Value_::U8(u8_val) => {
                vec![
                    MOVE_ZERO_U8.to_owned(),
                    MOVE_MAX_U8.to_owned(),
                    u8_val.saturating_add(1).to_string(),
                    u8_val.saturating_sub(1).to_string(),
                ]
            },
            Value_::U16(u16_val) => {
                vec![
                    MOVE_ZERO_U16.to_owned(),
                    MOVE_MAX_U16.to_owned(),
                    u16_val.saturating_add(1).to_string(),
                    u16_val.saturating_sub(1).to_string(),
                ]
            },
            Value_::U32(u32_val) => {
                vec![
                    MOVE_ZERO_U32.to_owned(),
                    MOVE_MAX_U32.to_owned(),
                    u32_val.saturating_add(1).to_string(),
                    u32_val.saturating_sub(1).to_string(),
                ]
            },
            Value_::U64(u64_val) => {
                vec![
                    MOVE_ZERO_U64.to_owned(),
                    MOVE_MAX_U64.to_owned(),
                    u64_val.saturating_add(1).to_string(),
                    u64_val.saturating_sub(1).to_string(),
                ]
            },
            Value_::U128(u128_val) => {
                vec![
                    MOVE_ZERO_U128.to_owned(),
                    MOVE_MAX_U128.to_owned(),
                    u128_val.saturating_add(1).to_string(),
                    u128_val.saturating_sub(1).to_string(),
                ]
            },
            Value_::U256(_u256_val) => {
                vec![MOVE_ZERO_U256.to_owned(), MOVE_MAX_U256.to_owned()]
            },
            Value_::InferredNum(_inferred_num) => {
                vec![MOVE_ZERO.to_owned(), MOVE_MAX_INFERRED_NUM.to_owned()]
            },
        };

        ops.into_iter()
            .filter(|v| cur_op != *v)
            .map(|op| {
                let mut mutated_source = source.to_string();
                mutated_source.replace_range(start..end, op.as_str());
                MutantInfo::new(
                    mutated_source,
                    Mutation::new(
                        Range::new(start, end),
                        OPERATOR_NAME.to_string(),
                        cur_op.to_string(),
                        op.to_string(),
                    ),
                )
            })
            .collect()
    }

    fn get_file_hash(&self) -> FileHash {
        self.operation.loc.file_hash()
    }

    fn name(&self) -> String {
        OPERATOR_NAME.to_string()
    }
}

impl fmt::Display for Literal {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "LiteralOperator({:?}, location: file hash: {}, index start: {}, index stop: {})",
            self.operation.value,
            self.operation.loc.file_hash(),
            self.operation.loc.start(),
            self.operation.loc.end()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use move_command_line_common::files::FileHash;
    use move_compiler::expansion::ast::{Value, Value_};
    use move_ir_types::location::Loc;

    #[test]
    fn test_apply_u8() {
        let loc = Loc::new(FileHash::new(""), 0, 2);
        let val = Value {
            value: Value_::U8(51),
            loc,
        };
        let operator = Literal::new(val);
        let source = "51";
        let expected = vec![MOVE_ZERO_U8, MOVE_MAX_U8, "52", "50"];
        let result = operator.apply(source);
        assert_eq!(result.len(), expected.len());
        for (i, r) in result.iter().enumerate() {
            assert_eq!(r.mutated_source, expected[i]);
        }
    }

    #[test]
    fn test_apply_u16() {
        let loc = Loc::new(FileHash::new(""), 0, 3);
        let val = Value {
            value: Value_::U16(963),
            loc,
        };
        let operator = Literal::new(val);
        let source = "963";
        let expected = vec![MOVE_ZERO_U16, MOVE_MAX_U16, "964", "962"];
        let result = operator.apply(source);
        assert_eq!(result.len(), expected.len());
        for (i, r) in result.iter().enumerate() {
            assert_eq!(r.mutated_source, expected[i]);
        }
    }

    #[test]
    fn test_apply_u32() {
        let loc = Loc::new(FileHash::new(""), 0, 7);
        let val = Value {
            value: Value_::U32(1000963),
            loc,
        };
        let operator = Literal::new(val);
        let source = "1000963";
        let expected = vec![MOVE_ZERO_U32, MOVE_MAX_U32, "1000964", "1000962"];
        let result = operator.apply(source);
        assert_eq!(result.len(), expected.len());
        for (i, r) in result.iter().enumerate() {
            assert_eq!(r.mutated_source, expected[i]);
        }
    }

    #[test]
    fn test_apply_u64() {
        let loc = Loc::new(FileHash::new(""), 0, 12);
        let val = Value {
            value: Value_::U64(963251000963),
            loc,
        };
        let operator = Literal::new(val);
        let source = "963251000963";
        let expected = vec![MOVE_ZERO_U64, MOVE_MAX_U64, "963251000964", "963251000962"];
        let result = operator.apply(source);
        assert_eq!(result.len(), expected.len());
        for (i, r) in result.iter().enumerate() {
            assert_eq!(r.mutated_source, expected[i]);
        }
    }

    #[test]
    fn test_apply_u128() {
        let loc = Loc::new(FileHash::new(""), 0, 15);
        let val = Value {
            value: Value_::U128(123963251000963),
            loc,
        };
        let operator = Literal::new(val);
        let source = "123963251000963";
        let expected = vec![
            MOVE_ZERO_U128,
            MOVE_MAX_U128,
            "123963251000964",
            "123963251000962",
        ];
        let result = operator.apply(source);
        assert_eq!(result.len(), expected.len());
        for (i, r) in result.iter().enumerate() {
            assert_eq!(r.mutated_source, expected[i]);
        }
    }

    #[test]
    fn test_apply_bool() {
        let loc = Loc::new(FileHash::new(""), 0, 4);
        let val = Value {
            value: Value_::Bool(true),
            loc,
        };
        let operator = Literal::new(val);
        let source = MOVE_TRUE;
        let expected = vec![MOVE_FALSE];
        let result = operator.apply(source);
        assert_eq!(result.len(), expected.len());
        for (i, r) in result.iter().enumerate() {
            assert_eq!(r.mutated_source, expected[i]);
        }
    }

    #[test]
    fn test_get_file_hash() {
        let loc = Loc::new(FileHash::new(""), 0, 0);
        let val = Value {
            value: Value_::U8(99),
            loc,
        };
        let operator = Literal::new(val);
        assert_eq!(operator.get_file_hash(), FileHash::new(""));
    }
}
