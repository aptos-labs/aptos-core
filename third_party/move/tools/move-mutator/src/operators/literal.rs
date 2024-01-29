use crate::operator::{MutantInfo, MutationOperator};
use crate::report::{Mutation, Range};
use move_command_line_common::files::FileHash;
use move_compiler::expansion::ast;
use move_compiler::expansion::ast::Value_;
use std::fmt;
use std::fmt::Debug;

/// Break and continue mutation operator.
/// Replaces break and continue statements with each other or deletes them.
#[derive(Debug, Clone)]
pub struct Literal {
    operation: ast::Value,
}

impl Literal {
    /// Creates a new instance of the break/continue mutation operator.
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

        // Group of exchangeable break/continue statements.
        let ops: Vec<&str> = match &self.operation.value {
            Value_::Address(_addr) => {
                vec![
                    "0x0",
                    "0xFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF",
                ]
            },
            Value_::Bool(_bool_val) => {
                vec!["true", "false"]
            },
            Value_::Bytearray(_bytearray) => {
                vec!["b\"121112\""]
            },
            Value_::U8(_u8_val) => {
                vec!["0u8", "255u8"]
            },
            Value_::U16(_u16_val) => {
                vec!["0u16", "65535u16"]
            },
            Value_::U32(_u32_val) => {
                vec!["0u32", "4294967295u32"]
            },
            Value_::U64(_u64_val) => {
                vec!["0u64", "18446744073709551615u64"]
            },
            Value_::U128(_u128_val) => {
                vec!["0u128", "340282366920938463463374607431768211455u128"]
            },
            Value_::U256(_u256_val) => {
                vec!["0u256", "115792089237316195423570985008687907853269984665640564039457584007913129639935u256"]
            },
            Value_::InferredNum(_inferred_num) => {
                vec!["0", "115792089237316195423570985008687907853269984665640564039457584007913129639935"]
            },
        };

        ops.into_iter()
            .filter(|v| cur_op != *v)
            .map(|op| {
                let mut mutated_source = source.to_string();
                mutated_source.replace_range(start..end, op);
                MutantInfo::new(
                    mutated_source,
                    Mutation::new(
                        Range::new(start, end),
                        "literal_replacement".to_string(),
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
            value: Value_::U8(99),
            loc,
        };
        let operator = Literal::new(val);
        let source = "51";
        let expected = vec!["0u8", "255u8"];
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
        let expected = vec!["0u16", "65535u16"];
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
        let expected = vec!["0u32", "4294967295u32"];
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
        let expected = vec!["0u64", "18446744073709551615u64"];
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
        let expected = vec!["0u128", "340282366920938463463374607431768211455u128"];
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
        let source = "true";
        let expected = vec!["false"];
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
