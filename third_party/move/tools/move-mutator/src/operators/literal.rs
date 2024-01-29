use crate::operator::{MutantInfo, MutationOperator};
use crate::report::{Mutation, Range};
use move_command_line_common::files::FileHash;
use move_compiler::expansion::ast;
use move_compiler::expansion::ast::Value_;
use std::fmt;
use std::fmt::Debug;

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
                vec![
                    "0x0".to_owned(),
                    "0xFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF".to_owned(),
                ]
            },
            Value_::Bool(_bool_val) => {
                vec!["true".to_owned(), "false".to_owned()]
            },
            Value_::Bytearray(_bytearray) => {
                vec!["b\"121112\"".to_owned()]
            },
            Value_::U8(u8_val) => {
                vec![
                    "0u8".to_owned(),
                    "255u8".to_owned(),
                    (u8_val + 1).to_string(),
                    (u8_val - 1).to_string(),
                ]
            },
            Value_::U16(u16_val) => {
                vec![
                    "0u16".to_owned(),
                    "65535u16".to_owned(),
                    (u16_val + 1).to_string(),
                    (u16_val - 1).to_string(),
                ]
            },
            Value_::U32(u32_val) => {
                vec![
                    "0u32".to_owned(),
                    "4294967295u32".to_owned(),
                    (u32_val + 1).to_string(),
                    (u32_val - 1).to_string(),
                ]
            },
            Value_::U64(u64_val) => {
                vec![
                    "0u64".to_owned(),
                    "18446744073709551615u64".to_owned(),
                    (u64_val + 1).to_string(),
                    (u64_val - 1).to_string(),
                ]
            },
            Value_::U128(u128_val) => {
                vec![
                    "0u128".to_owned(),
                    "340282366920938463463374607431768211455u128".to_owned(),
                    (u128_val + 1).to_string(),
                    (u128_val - 1).to_string(),
                ]
            },
            Value_::U256(_u256_val) => {
                vec!["0u256".to_owned(), "115792089237316195423570985008687907853269984665640564039457584007913129639935u256".to_owned()]
            },
            Value_::InferredNum(_inferred_num) => {
                vec!["0".to_owned(), "115792089237316195423570985008687907853269984665640564039457584007913129639935".to_owned()]
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
            value: Value_::U8(51),
            loc,
        };
        let operator = Literal::new(val);
        let source = "51";
        let expected = vec!["0u8", "255u8", "52", "50"];
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
        let expected = vec!["0u16", "65535u16", "964", "962"];
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
        let expected = vec!["0u32", "4294967295u32", "1000964", "1000962"];
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
        let expected = vec!["0u64", "18446744073709551615u64", "963251000964", "963251000962"];
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
        let expected = vec!["0u128", "340282366920938463463374607431768211455u128", "123963251000964", "123963251000962"];
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
