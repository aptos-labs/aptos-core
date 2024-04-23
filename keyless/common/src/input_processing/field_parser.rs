// Copyright © Aptos Foundation

use std::{iter::Peekable, str::CharIndices};
use thiserror::Error;

#[derive(Debug, PartialEq, Eq)]
pub struct ParsedField<IndexInJwt> {
    pub index: IndexInJwt,
    pub key: String,
    pub value: String,
    pub colon_index: usize,
    pub value_index: usize,
    pub whole_field: String,
}

#[derive(Debug, PartialEq, Eq)]
pub struct IndexInJwtNotSet {}

pub type ConsumeResultStr = Result<(usize, String), FieldParserError>;
pub type ConsumeResultChar = Result<(usize, char), FieldParserError>;
pub type ConsumeResultEmpty = Result<(usize, ()), FieldParserError>;

#[derive(Debug, PartialEq, Eq, Error)]
#[error(
    "Parse error. {}. Occurred at index {} of {}",
    explanation,
    index,
    whole_str
)]
pub struct FieldParserError {
    explanation: String,
    index: usize,
    whole_str: String,
}

#[derive(Debug)]
pub struct FieldParser<'a> {
    char_indices: Peekable<CharIndices<'a>>,
    whole_str: String,
}

impl<'a> FieldParser<'a> {
    pub fn new(s: &'a str) -> Self {
        let char_indices = s.char_indices().peekable();
        Self {
            char_indices,
            whole_str: String::from(s),
        }
    }

    pub fn error(&mut self, explanation: &str) -> FieldParserError {
        FieldParserError {
            explanation: String::from(explanation),
            index: match self.char_indices.peek() {
                Some((i, _)) => *i,
                None => self.whole_str.len(),
            },
            whole_str: String::from(&self.whole_str),
        }
    }

    pub fn eos_error(&mut self) -> FieldParserError {
        self.error("Unexpected end of stream")
    }

    pub fn parse(&mut self) -> Result<ParsedField<IndexInJwtNotSet>, FieldParserError> {
        let (_, key) = self.consume_string()?;
        let (colon_index, _) = self.consume_non_whitespace_char(&[':'])?;
        let (value_index, value) = self.consume_value()?;
        let (field_end_delimiter_index, _) = self.consume_non_whitespace_char(&[',', '}'])?;
        let whole_field = String::from(&self.whole_str[..field_end_delimiter_index + 1]);

        Ok(ParsedField {
            index: IndexInJwtNotSet {},
            // key should not have quotes
            key,
            value,
            colon_index,
            value_index,
            whole_field,
        })
    }

    pub fn peek(&mut self) -> ConsumeResultChar {
        match self.char_indices.peek() {
            Some(p) => Ok(*p),
            None => Err(self.eos_error()),
        }
    }

    #[allow(clippy::all)]
    pub fn next(&mut self) -> ConsumeResultChar {
        self.char_indices.next().ok_or_else(|| self.eos_error())
    }

    pub fn consume_whitespace(&mut self) -> ConsumeResultEmpty {
        let (index, _) = self.peek()?;
        while self.peek()?.1 == ' ' {
            self.next().map_err(|_| self.eos_error())?;
        }
        Ok((index, ()))
    }

    pub fn consume_non_whitespace_char(&mut self, char_options: &[char]) -> ConsumeResultChar {
        while self.peek()?.1 == ' ' {
            self.next().map_err(|_| self.eos_error())?;
        }

        let c = self.peek()?.1;
        if char_options.contains(&c) {
            self.next().map_err(|_| self.eos_error())
        } else {
            Err(self.error(&format!(
                "Expected a character in {:?}, got {}",
                char_options, c
            )))
        }
    }

    fn consume_value(&mut self) -> ConsumeResultStr {
        self.consume_whitespace()?;
        match self.peek()?.1 {
            '"' => self.consume_string(),
            _ => self.consume_unquoted(),
        }
    }

    // Should this handle escaped characters (e.g., quotes, newlines)? It doesn't currently.
    fn consume_string(&mut self) -> ConsumeResultStr {
        if self.peek()?.1 != '"' {
            Err(self.error("Expected a string here"))
        } else {
            self.next()?; // ignore the '"'

            let (index, _) = self.peek()?;
            let mut result = String::new();

            result.push(self.next()?.1); // push the '"'

            while self.peek()?.1 != '"' {
                result.push(self.next()?.1);
            }

            self.next()?; // ignore the '"'

            // The circuit requires the value_index to be for the first character after the quote.
            Ok((index, result))
        }
    }

    fn consume_unquoted(&mut self) -> ConsumeResultStr {
        let (index, _) = self.peek()?;
        let mut result = String::new();

        while self.peek()?.1 != ' ' && self.peek()?.1 != ',' && self.peek()?.1 != '}' {
            result.push(self.next()?.1);
        }

        Ok((index, result))
    }

    pub fn find_and_parse_field(
        jwt_payload: &'a str,
        key: &str,
    ) -> Result<ParsedField<usize>, FieldParserError> {
        let key_in_quotes = String::from("\"") + key + "\"";

        let index = jwt_payload
            .find(&key_in_quotes)
            .ok_or_else(|| FieldParserError {
                explanation: format!(
                    "Could not find {} in jwt payload: {}",
                    key_in_quotes, jwt_payload
                ),
                index: 0,
                whole_str: String::from(jwt_payload),
            })?;

        let field_check_input = Self::new(&jwt_payload[index..]).parse()?;

        Ok(ParsedField {
            index,
            key: field_check_input.key,
            value: field_check_input.value,
            colon_index: field_check_input.colon_index,
            value_index: field_check_input.value_index,
            whole_field: field_check_input.whole_field,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::{FieldParserError, IndexInJwtNotSet, ParsedField};
    use crate::input_processing::field_parser::FieldParser;

    // TODO other test cases to potentially use
    //    const TEST_FIELDS : [&'static str; 16] = [
    //        "\"iss\": \"https://accounts.google.com\",",
    //        "\"azp\" : \"407408718192.apps.googleusercontent.com\",",
    //        "\"aud\":\"407408718192.apps.googleusercontent.com\",",
    //        "\"sub\"   :   \"113990307082899718775\",",
    //        "\"hd\": \"aptoslabs.com\" ,",
    //        "\"email\": \"michael@aptoslabs.com\" , DONTINCLUDETHISINRESULT",
    //        "\"email_verified\": true,",
    //        "\"at_hash\": \"bxIESuI59IoZb5alCASqBg\",",
    //        "\"name\": \"Michael Straka\",",
    //        "\"picture\": \"https://lh3.googleusercontent.com/a/ACg8ocJvY4kVUBRtLxe1IqKWL5i7tBDJzFp9YuWVXMzwPpbs=s96-c\",",
    //        "\"given_name\": \"Michael\",",
    //        "\"family_name\": \"Straka\",",
    //        "\"locale\": \"en\",",
    //        "\"exp\":2700259544}",
    //        "\"exp\":2700259544 \"",
    //        "\"sub   :   \"113990307082899718775\",",
    //    ];

    fn success(
        key: &str,
        value: &str,
        colon_index: usize,
        value_index: usize,
        whole_field: &str,
    ) -> Result<ParsedField<IndexInJwtNotSet>, FieldParserError> {
        Ok(ParsedField {
            index: IndexInJwtNotSet {},
            key: String::from(key),
            value: String::from(value),
            colon_index,
            value_index,
            whole_field: String::from(whole_field),
        })
    }

    #[test]
    fn test_parse_iss() {
        let result = FieldParser::new("\"iss\": \"https://accounts.google.com\",").parse();

        assert_eq!(
            result,
            success(
                "iss",
                "https://accounts.google.com",
                5,
                8,
                "\"iss\": \"https://accounts.google.com\","
            )
        );
    }

    #[test]
    fn test_parse_email_extra_chars() {
        let result =
            FieldParser::new("\"email\": \"michael@aptoslabs.com\" , DONTINCLUDETHISINRESULT")
                .parse();

        println!("{:?}", result);

        assert_eq!(
            result,
            success(
                "email",
                "michael@aptoslabs.com",
                7,
                10,
                "\"email\": \"michael@aptoslabs.com\" ,"
            )
        );
    }
}
