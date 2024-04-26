// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::TestCircuitHandle;
use crate::misc::calc_string_bodies;
use aptos_keyless_common::input_processing::{
    circuit_input_signals::{CircuitInputSignals, Padded}, config::CircuitPaddingConfig,
};

struct JWTField<T> {
    whole_field: T,
    name: T,
    value: T,
}

struct JWTFieldMaliciousIndices<T> {
    whole_field: T,
    name: T,
    value: T,
    whole_field_len: usize,
    name_len: usize,
    value_index: usize,
    value_len: usize,
    colon_index: usize,
}

trait JWTFieldIndices {
    fn whole_field_len(&self) -> usize;
    fn name_len(&self) -> usize;
    fn value_index(&self) -> usize;
    fn value_len(&self) -> usize;
    fn colon_index(&self) -> usize;
}

trait JWTFieldStr {
    fn whole_field(&self) -> &str;
    fn name(&self) -> &str;
    fn value(&self) -> &str;
}

impl JWTFieldIndices for JWTField<String> {
    fn whole_field_len(&self) -> usize {
        self.whole_field.len()
    }

    fn name_len(&self) -> usize {
        self.name.len()
    }

    fn value_index(&self) -> usize {
        self.whole_field.find(&self.value).unwrap()
    }

    fn value_len(&self) -> usize {
        self.value.len()
    }

    fn colon_index(&self) -> usize {
        self.whole_field.find(':').unwrap()
    }
}

impl JWTFieldStr for JWTField<String> {
    fn whole_field(&self) -> &str {
        &self.whole_field
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn value(&self) -> &str {
        &self.value
    }
}


impl JWTFieldIndices for JWTFieldMaliciousIndices<String> {
    fn whole_field_len(&self) -> usize {
        self.whole_field_len
    }

    fn name_len(&self) -> usize {
        self.name_len
    }

    fn value_index(&self) -> usize {
        self.value_index
    }

    fn value_len(&self) -> usize {
        self.value_len
    }

    fn colon_index(&self) -> usize {
        self.colon_index
    }
}

impl JWTFieldStr for JWTFieldMaliciousIndices<String> {
    fn whole_field(&self) -> &str {
        &self.whole_field
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn value(&self) -> &str {
        &self.value
    }
}

fn jwt_field_str
(
    whole_field: &str,
    name: &str,
    value: &str,
)
-> JWTField<String> {
    JWTField {
        whole_field: String::from(whole_field),
        name: String::from(name),
        value: String::from(value),
    }
}

fn jwt_field_str_malicious_indices
(
    whole_field: &str,
    name: &str,
    value: &str,
)
-> JWTFieldMaliciousIndices<String> {
    JWTFieldMaliciousIndices {
        whole_field: String::from(whole_field),
        name: String::from(name),
        value: String::from(value),
        whole_field_len: whole_field.len(),
        name_len: name.len(),
        value_index: whole_field.find(value).unwrap_or(0),
        value_len: value.len(),
        colon_index: whole_field.find(':').unwrap()
    }
}

fn prepare_jwt_field_check_test_str<T: JWTFieldIndices + JWTFieldStr>(field: T) -> CircuitInputSignals<Padded> {
    let config = CircuitPaddingConfig::new()
        .max_length("field", 60)
        .max_length("name", 30)
        .max_length("value", 30);

    CircuitInputSignals::new()
        .str_input("field", &field.whole_field())
        .str_input("name", &field.name())
        .str_input("value", &field.value())
        .usize_input("field_len", field.whole_field_len())
        .usize_input("field_index", 0)
        .bools_input("string_bodies", &calc_string_bodies(field.whole_field()))
        .usize_input("name_len", field.name_len())
        .usize_input("value_index", field.value_index())
        .usize_input("value_len", field.value_len())
        .usize_input("colon_index", field.colon_index())
        .pad(&config)
        .unwrap()
}

fn should_pass<T: JWTFieldIndices + JWTFieldStr>(field: T, test_circom_file: &str) {
    let circuit_handle = TestCircuitHandle::new(test_circom_file).unwrap();

    let circuit_input_signals = prepare_jwt_field_check_test_str(
        field
        );

    let result = circuit_handle.gen_witness(circuit_input_signals);
    println!("{:?}", result);
    assert!(result.is_ok());
}

fn should_fail<T: JWTFieldIndices + JWTFieldStr> (field: T, test_circom_file: &str) {
    let circuit_handle = TestCircuitHandle::new(test_circom_file).unwrap();

    let circuit_input_signals = prepare_jwt_field_check_test_str(
        field
        );

    let result = circuit_handle.gen_witness(circuit_input_signals);
    println!("{:?}", result);
    assert!(result.is_err());
}

fn should_pass_quoted<T: JWTFieldIndices + JWTFieldStr>(field: T) {
    should_pass(field, "jwt_field_parsing/parse_quoted_test.circom");
}

fn should_pass_unquoted<T: JWTFieldIndices + JWTFieldStr>(field: T) {
    should_pass(field, "jwt_field_parsing/parse_unquoted_test.circom");
}


fn should_fail_quoted<T: JWTFieldIndices + JWTFieldStr>(field: T) {
    should_fail(field, "jwt_field_parsing/parse_quoted_test.circom");
}

fn should_fail_unquoted<T: JWTFieldIndices + JWTFieldStr>(field: T) {
    should_fail(field, "jwt_field_parsing/parse_unquoted_test.circom");
}






// The tests

#[test]
fn simple_quoted() {
   should_pass_quoted(
        jwt_field_str(
            "\"name\": \"value\",",
            "name",
            "value",
            )
        );
}



#[test]
fn simple_unquoted() {
   should_pass_unquoted(
        jwt_field_str(
            "\"name\": value,",
            "name",
            "value",
            )
        );
}

#[test]
fn no_whitespace_quoted() {
   should_pass_quoted(
        jwt_field_str(
            "\"name\":\"value\",",
            "name",
            "value",
            )
        );
}


#[test]
fn no_whitespace_unquoted() {
   should_pass_unquoted(
        jwt_field_str(
            "\"name\":value,",
            "name",
            "value",
            )
        );
}

#[test]
fn malicious_value_1() {
    let mut field = jwt_field_str_malicious_indices(
            "\"sub\": \"a\\\",b\",",
            "sub",
            "a\\",
            );
    field.whole_field_len = field.whole_field.find(',').unwrap()+1;
   should_pass_quoted(field);
}

#[test]
fn malicious_value_2() {
   should_pass_quoted(
        jwt_field_str(
            "\"name1\":\"value1\",\"name2\":\"value2\",",
            "name1",
            "value1\",\"name2\":\"value2",
            )
        );
}


#[test]
fn end_with_curly_bracket() {
   should_pass_quoted(
        jwt_field_str(
            "\"name\": \"value\"}",
            "name",
            "value",
            )
        );
}

#[test]
fn end_with_curly_bracket_unquoted() {
   should_pass_unquoted(
        jwt_field_str(
            "\"name\": value}",
            "name",
            "value",
            )
        );
}


#[test]
fn should_fail_when_name_has_no_first_quote() {
       should_fail_quoted(
        jwt_field_str(
            "name\": \"value\",",
            "name",
            "value",
            )
        );
}

#[test]
fn should_fail_when_name_has_no_second_quote() {
    should_fail_quoted(
        jwt_field_str(
            "\"name: \"value\",",
            "name",
            "value",
            )
        );
}

#[test]
fn should_fail_when_name_has_no_quotes() {
    should_fail_quoted(
        jwt_field_str(
            "name: \"value\",",
            "name",
            "value",
            )
        );
}

#[test]
fn should_fail_when_name_not_equal_quoted() {
   should_fail_quoted(
        jwt_field_str(
            "\"name\": \"value\",",
            "fake",
            "value",
            )
        );
}

#[test]
fn should_fail_when_name_not_equal_unquoted() {
   should_fail_unquoted(
        jwt_field_str(
            "\"name\": value,",
            "fake",
            "value",
            )
        );
}

#[test]
fn should_fail_when_value_not_equal_quoted() {
    let mut field = 
        jwt_field_str_malicious_indices(
            "\"name\": \"value\",",
            "name",
            "fake",
            );
    field.whole_field_len = field.whole_field.len();
    field.value_index = field.whole_field.find("value").unwrap();
    should_fail_quoted(field);
}

#[test]
fn should_fail_when_value_not_equal_unquoted() {
    let mut field = 
        jwt_field_str_malicious_indices(
            "\"name\": value,",
            "name",
            "fake",
            );
    field.whole_field_len = field.whole_field.len();
    field.value_index = field.whole_field.find("value").unwrap();
    should_fail_unquoted(field);
}


// ref: Circuit Bug #3, https://www.notion.so/aptoslabs/JWTFieldCheck-does-not-properly-constrain-field_len-which-can-cause-the-circuit-to-accept-field-val-9943c152e7274f35a1669a6cb416c7bf?pvs=4
#[test]
fn malicious_field_len() {
    let mut field = jwt_field_str_malicious_indices(
        "\"name\":\",value\"",
        "name",
        ",value",
        );
    field.whole_field_len = field.whole_field.find(",").unwrap() + 1;
    field.value_index = field.whole_field.find(",").unwrap();
    assert_ne!(field.whole_field_len, field.whole_field.len());

    should_fail_quoted(field);
}

// ref: Circuit Bug #4, https://www.notion.so/aptoslabs/JWTFieldCheck-allows-for-maliciously-truncating-field-values-at-any-character-f8695dcd397a4bc2b66d52349388499f?pvs=4
#[test]
fn malicious_value_len_1() {
    let mut field = jwt_field_str_malicious_indices(
        "\"sub\":\"user,fake\",",
        "sub",
        "user",
        );

    field.whole_field_len = field.whole_field.find(",").unwrap() + 1;

    should_fail_quoted(field);
}

#[test]
fn malicious_value_len_2() {
    let mut field = jwt_field_str_malicious_indices(
        "\"sub\":user,fake,",
        "sub",
        "user",
        );

    field.whole_field_len = field.whole_field.find(",").unwrap() + 1;

    should_pass_unquoted(field);

    let mut field = jwt_field_str_malicious_indices(
        "\"sub\":user,fake,",
        "sub",
        "user,fake",
        );

    should_fail_unquoted(field);
}
