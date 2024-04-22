use aptos_keyless_common::input_processing::circuit_input_signals::CircuitInputSignals;
use aptos_keyless_common::input_processing::config::CircuitPaddingConfig;
use aptos_keyless_common::input_processing::witness_gen::witness_gen;
use crate::{TestCircuitHandle};
use itertools::*;
use aptos_crypto::poseidon_bn254;
use ark_bn254::Fr;
use ark_ff::{Zero, One};

fn build_array_selector_output(len: u32, start: u32, end: u32) -> Vec<u8> {
    let mut output = Vec::new();
    for _ in 0..start {
        output.push(0);
    };
    for _ in start..end {
        output.push(1);
    };
    for _ in end..len {
        output.push(0);
    };
    output
}
     
#[test]
fn array_selector_test() {
    let circuit_handle = TestCircuitHandle::new("array_selector_test.circom").unwrap();
    let out_len = 8;
    for start in 0..out_len {
        for end in start+1..out_len {
            let output = build_array_selector_output(out_len, start, end);
            let config = CircuitPaddingConfig::new().max_length("expected_output", out_len as usize);
            let circuit_input_signals = CircuitInputSignals::new().u64_input("start_index", start as u64).u64_input("end_index", end as u64).bytes_input("expected_output", &output[..]).pad(&config).unwrap();
            let result = circuit_handle.gen_witness(circuit_input_signals);
            assert!(result.is_ok());
        };
    };
}

#[test]
fn array_selector_test_large() {
    let circuit_handle = TestCircuitHandle::new("array_selector_test_large.circom").unwrap();
    let out_len = 2000;
    let start = 146;
    let end = 1437;
    let output = build_array_selector_output(out_len, start, end);
    let config = CircuitPaddingConfig::new().max_length("expected_output", out_len as usize);
    let circuit_input_signals = CircuitInputSignals::new().u64_input("start_index", start as u64).u64_input("end_index", end as u64).bytes_input("expected_output", &output).pad(&config).unwrap();
     
    let result = circuit_handle.gen_witness(circuit_input_signals);
    assert!(result.is_ok());
}

#[test]
fn array_selector_test_small() {
    let circuit_handle = TestCircuitHandle::new("array_selector_test_small.circom").unwrap();
    let out_len = 2;
    let start = 0;
    let end = 1;
    let output = build_array_selector_output(out_len, start, end);
    let config = CircuitPaddingConfig::new().max_length("expected_output", out_len as usize);
    let circuit_input_signals = CircuitInputSignals::new().u64_input("start_index", start as u64).u64_input("end_index", end as u64).bytes_input("expected_output", &output).pad(&config).unwrap();
     
    let result = circuit_handle.gen_witness(circuit_input_signals);
    assert!(result.is_ok());
}

#[test]
#[should_panic]
fn array_selector_test_wrong_end() {
    let circuit_handle = TestCircuitHandle::new("array_selector_test.circom").unwrap();
    let out_len = 8;
    let start = 3;
    let end = 8;
    let output = build_array_selector_output(out_len, start, end);
    let config = CircuitPaddingConfig::new().max_length("expected_output", out_len as usize);
    let circuit_input_signals = CircuitInputSignals::new().u64_input("start_index", start as u64).u64_input("end_index", end as u64).bytes_input("expected_output", &output).pad(&config).unwrap();
     
    let result = circuit_handle.gen_witness(circuit_input_signals);
    assert!(result.is_ok());
}

#[test]
#[should_panic]
fn array_selector_test_wrong_start() {
    let circuit_handle = TestCircuitHandle::new("array_selector_test.circom").unwrap();
    let out_len = 8;
    let start = 3;
    let end = 3;
    let output = build_array_selector_output(out_len, start, end);
    let config = CircuitPaddingConfig::new().max_length("expected_output", out_len as usize);
    let circuit_input_signals = CircuitInputSignals::new().u64_input("start_index", start as u64).u64_input("end_index", end as u64).bytes_input("expected_output", &output).pad(&config).unwrap();
     
    let result = circuit_handle.gen_witness(circuit_input_signals);
    assert!(result.is_ok());
}

#[test]
fn array_selector_test_complex() {
    let circuit_handle = TestCircuitHandle::new("array_selector_complex_test.circom").unwrap();
    let out_len = 8;
    // Fails when start = 0 by design
    for start in 1..out_len {
        for end in start+1..out_len {
            let output = build_array_selector_output(out_len, start, end);
            let config = CircuitPaddingConfig::new().max_length("expected_output", out_len as usize);
            let circuit_input_signals = CircuitInputSignals::new().u64_input("start_index", start as u64).u64_input("end_index", end as u64).bytes_input("expected_output", &output[..]).pad(&config).unwrap();
            let result = circuit_handle.gen_witness(circuit_input_signals);
            assert!(result.is_ok());
        };
    };
}

#[test]
fn array_selector_test_complex_large() {
    let circuit_handle = TestCircuitHandle::new("array_selector_complex_large_test.circom").unwrap();
    let out_len = 2000;
    let start = 157;
    let end = 1143;
    let output = build_array_selector_output(out_len, start, end);
    let config = CircuitPaddingConfig::new().max_length("expected_output", out_len as usize);
    let circuit_input_signals = CircuitInputSignals::new().u64_input("start_index", start as u64).u64_input("end_index", end as u64).bytes_input("expected_output", &output).pad(&config).unwrap();
     
    let result = circuit_handle.gen_witness(circuit_input_signals);
    assert!(result.is_ok());
}

#[test]
fn array_selector_test_complex_small() {
    let circuit_handle = TestCircuitHandle::new("array_selector_complex_small_test.circom").unwrap();
    let out_len = 3;
    let start = 1;
    let end = 2;
    let output = build_array_selector_output(out_len, start, end);
    let config = CircuitPaddingConfig::new().max_length("expected_output", out_len as usize);
    let circuit_input_signals = CircuitInputSignals::new().u64_input("start_index", start as u64).u64_input("end_index", end as u64).bytes_input("expected_output", &output).pad(&config).unwrap();
     
    let result = circuit_handle.gen_witness(circuit_input_signals);
    assert!(result.is_ok());
}

#[test]
#[should_panic]
fn array_selector_test_complex_wrong_end() {
    let circuit_handle = TestCircuitHandle::new("array_selector_complex_test.circom").unwrap();
    let out_len = 8;
    let start = 3;
    let end = 8;
    let output = build_array_selector_output(out_len, start, end);
    let config = CircuitPaddingConfig::new().max_length("expected_output", out_len as usize);
    let circuit_input_signals = CircuitInputSignals::new().u64_input("start_index", start as u64).u64_input("end_index", end as u64).bytes_input("expected_output", &output).pad(&config).unwrap();
     
    let result = circuit_handle.gen_witness(circuit_input_signals);
    assert!(result.is_ok());
}

#[test]
#[should_panic]
fn array_selector_test_complex_wrong_start() {
    let circuit_handle = TestCircuitHandle::new("array_selector_test.circom").unwrap();
    let out_len = 8;
    let start = 3;
    let end = 3;
    let output = build_array_selector_output(out_len, start, end);
    let config = CircuitPaddingConfig::new().max_length("expected_output", out_len as usize);
    let circuit_input_signals = CircuitInputSignals::new().u64_input("start_index", start as u64).u64_input("end_index", end as u64).bytes_input("expected_output", &output).pad(&config).unwrap();
     
    let result = circuit_handle.gen_witness(circuit_input_signals);
    assert!(result.is_ok());
}

fn build_left_array_selector_output(len: u32, index: u32) -> Vec<u8> {
    let mut output = Vec::new();
    for _ in 0..index {
        output.push(1);
    };
    for _ in index..len {
        output.push(0);
    };
    output
}

#[test]
fn left_array_selector_test() {
    let circuit_handle = TestCircuitHandle::new("left_array_selector_test.circom").unwrap();
    let out_len = 8;
    for index in 0..out_len {
        let output = build_left_array_selector_output(out_len, index);
        let config = CircuitPaddingConfig::new().max_length("expected_output", out_len as usize);
        let circuit_input_signals = CircuitInputSignals::new().u64_input("index", index as u64).bytes_input("expected_output", &output[..]).pad(&config).unwrap();
        let result = circuit_handle.gen_witness(circuit_input_signals);
        assert!(result.is_ok());
    };
}

#[test]
fn left_array_selector_test_large() {
    let circuit_handle = TestCircuitHandle::new("left_array_selector_large_test.circom").unwrap();
    let out_len = 2000;
    let index = 1143;
    let output = build_left_array_selector_output(out_len, index);
    let config = CircuitPaddingConfig::new().max_length("expected_output", out_len as usize);
    let circuit_input_signals = CircuitInputSignals::new().u64_input("index", index as u64).bytes_input("expected_output", &output).pad(&config).unwrap();
     
    let result = circuit_handle.gen_witness(circuit_input_signals);
    assert!(result.is_ok());
}

#[test]
fn left_array_selector_test_small() {
    let circuit_handle = TestCircuitHandle::new("left_array_selector_small_test.circom").unwrap();
    let out_len = 1;
    let index = 0;
    let output = build_left_array_selector_output(out_len, index);
    let config = CircuitPaddingConfig::new().max_length("expected_output", out_len as usize);
    let circuit_input_signals = CircuitInputSignals::new().u64_input("index", index as u64).bytes_input("expected_output", &output).pad(&config).unwrap();
     
    let result = circuit_handle.gen_witness(circuit_input_signals);
    assert!(result.is_ok());
}

#[test]
#[should_panic]
fn left_array_selector_test_wrong_index() {
    let circuit_handle = TestCircuitHandle::new("left_array_selector_test.circom").unwrap();
    let out_len = 8;
    let index = 8;
    let output = build_left_array_selector_output(out_len, index);
    let config = CircuitPaddingConfig::new().max_length("expected_output", out_len as usize);
    let circuit_input_signals = CircuitInputSignals::new().u64_input("index", index as u64).bytes_input("expected_output", &output).pad(&config).unwrap();
     
    let result = circuit_handle.gen_witness(circuit_input_signals);
    assert!(result.is_ok());
}

fn build_right_array_selector_output(len: u32, index: u32) -> Vec<u8> {
    let mut output = Vec::new();
    for _ in 0..index+1 {
        output.push(0);
    };
    for _ in index+1..len {
        output.push(1);
    };
    output
}

#[test]
fn right_array_selector_test() {
    let circuit_handle = TestCircuitHandle::new("right_array_selector_test.circom").unwrap();
    let out_len = 8;
    for index in 0..out_len {
        let output = build_right_array_selector_output(out_len, index);
        let config = CircuitPaddingConfig::new().max_length("expected_output", out_len as usize);
        let circuit_input_signals = CircuitInputSignals::new().u64_input("index", index as u64).bytes_input("expected_output", &output[..]).pad(&config).unwrap();
        let result = circuit_handle.gen_witness(circuit_input_signals);
        assert!(result.is_ok());
    };
}

#[test]
fn right_array_selector_test_large() {
    let circuit_handle = TestCircuitHandle::new("right_array_selector_large_test.circom").unwrap();
    let out_len = 2000;
    let index = 1143;
    let output = build_right_array_selector_output(out_len, index);
    let config = CircuitPaddingConfig::new().max_length("expected_output", out_len as usize);
    let circuit_input_signals = CircuitInputSignals::new().u64_input("index", index as u64).bytes_input("expected_output", &output).pad(&config).unwrap();
     
    let result = circuit_handle.gen_witness(circuit_input_signals);
    assert!(result.is_ok());
}

#[test]
fn right_array_selector_test_small() {
    let circuit_handle = TestCircuitHandle::new("right_array_selector_small_test.circom").unwrap();
    let out_len = 1;
    let index = 0;
    let output = build_left_array_selector_output(out_len, index);
    let config = CircuitPaddingConfig::new().max_length("expected_output", out_len as usize);
    let circuit_input_signals = CircuitInputSignals::new().u64_input("index", index as u64).bytes_input("expected_output", &output).pad(&config).unwrap();
     
    let result = circuit_handle.gen_witness(circuit_input_signals);
    assert!(result.is_ok());
}

#[test]
#[should_panic]
fn right_array_selector_test_wrong_index() {
    let circuit_handle = TestCircuitHandle::new("right_array_selector_test.circom").unwrap();
    let out_len = 8;
    let index = 8;
    let output = build_right_array_selector_output(out_len, index);
    let config = CircuitPaddingConfig::new().max_length("expected_output", out_len as usize);
    let circuit_input_signals = CircuitInputSignals::new().u64_input("index", index as u64).bytes_input("expected_output", &output).pad(&config).unwrap();
     
    let result = circuit_handle.gen_witness(circuit_input_signals);
    assert!(result.is_ok());
}

fn build_single_one_array_output(len: u32, index: u32) -> Vec<u8> {
    let mut output = Vec::new();
    for _ in 0..index {
        output.push(0);
    };
    output.push(1);
    for _ in index+2..len {
        output.push(0);
    };
    output
}

#[test]
fn single_one_array_test() {
    let circuit_handle = TestCircuitHandle::new("single_one_array_test.circom").unwrap();
    let out_len = 8;
    for index in 0..out_len {
        let output = build_single_one_array_output(out_len, index);
        let config = CircuitPaddingConfig::new().max_length("expected_output", out_len as usize);
        let circuit_input_signals = CircuitInputSignals::new().u64_input("index", index as u64).bytes_input("expected_output", &output).pad(&config).unwrap();
     
        let result = circuit_handle.gen_witness(circuit_input_signals);
        assert!(result.is_ok());
    }
}

#[test]
fn single_one_array_large_test() {
    let circuit_handle = TestCircuitHandle::new("single_one_array_large_test.circom").unwrap();
    let out_len = 2000;
    let index = 1143;
    let output = build_single_one_array_output(out_len, index);
    let config = CircuitPaddingConfig::new().max_length("expected_output", out_len as usize);
    let circuit_input_signals = CircuitInputSignals::new().u64_input("index", index as u64).bytes_input("expected_output", &output).pad(&config).unwrap();
     
    let result = circuit_handle.gen_witness(circuit_input_signals);
    assert!(result.is_ok());
}

#[test]
fn single_one_array_small_test() {
    let circuit_handle = TestCircuitHandle::new("single_one_array_small_test.circom").unwrap();
    let out_len = 1;
    let index = 0;
    let output = build_single_one_array_output(out_len, index);
    let config = CircuitPaddingConfig::new().max_length("expected_output", out_len as usize);
    let circuit_input_signals = CircuitInputSignals::new().u64_input("index", index as u64).bytes_input("expected_output", &output).pad(&config).unwrap();
     
    let result = circuit_handle.gen_witness(circuit_input_signals);
    assert!(result.is_ok());
}

#[test]
#[should_panic]
fn single_one_array_test_wrong_index() {
    let circuit_handle = TestCircuitHandle::new("single_one_array_test.circom").unwrap();
    let out_len = 8;
    let index = 8;
    let output = build_single_one_array_output(out_len, index);
    let config = CircuitPaddingConfig::new().max_length("expected_output", out_len as usize);
    let circuit_input_signals = CircuitInputSignals::new().u64_input("index", index as u64).bytes_input("expected_output", &output).pad(&config).unwrap();
     
    let result = circuit_handle.gen_witness(circuit_input_signals);
    assert!(result.is_ok());
}

#[test]
fn select_array_value_test() {
    let circuit_handle = TestCircuitHandle::new("select_array_value_test.circom").unwrap();
    let array = [4,6,1,8,9,4,2,3];
    let index = 4;
    let output = 9;
    let in_len = 8;
    let config = CircuitPaddingConfig::new().max_length("array", in_len);
    let circuit_input_signals = CircuitInputSignals::new().u64_input("index", index).bytes_input("array", &array).u64_input("expected_output", output).pad(&config).unwrap();
     
    let result = circuit_handle.gen_witness(circuit_input_signals);
    assert!(result.is_ok());
}

#[test]
fn single_neg_one_array_test() {
    let circuit_handle = TestCircuitHandle::new("single_neg_one_array_test.circom").unwrap();
    let output = [Fr::zero(), Fr::zero(), Fr::zero()-Fr::one(), Fr::zero(), Fr::zero(), Fr::zero(), Fr::zero(), Fr::zero()]; //[0,0,-1,0,0,0,0,0];
    let index = 2;
    let out_len = 8;
    let config = CircuitPaddingConfig::new().max_length("expected_output", out_len);
    let circuit_input_signals = CircuitInputSignals::new().u64_input("index", index).frs_input("expected_output", &output).pad(&config).unwrap();
     
    let result = circuit_handle.gen_witness(circuit_input_signals);
    assert!(result.is_ok());
}

#[test]
fn check_substr_inclusion_poly_test() {
    let circuit_handle = TestCircuitHandle::new("check_substr_inclusion_poly_test.circom").unwrap();

    let max_str_len = 100;
    let max_substr_len = 20;
    let config = CircuitPaddingConfig::new().max_length("str", max_str_len).max_length("substr", max_substr_len);
    let string = "Hello World!";
    let string_hash = poseidon_bn254::pad_and_hash_string(&string, max_str_len).unwrap();
    let substring = "lo Wor";
    let substring_len = substring.len();
    let start_index = 3;


    let circuit_input_signals = CircuitInputSignals::new().str_input("str", string).str_input("substr", substring).u64_input("substr_len", substring_len as u64).u64_input("start_index", start_index).fr_input("str_hash", string_hash).pad(&config).unwrap();

     
    let result = circuit_handle.gen_witness(circuit_input_signals);
    assert!(result.is_ok());
}

#[test]
fn check_substr_inclusion_poly_boolean_test() {
    let circuit_handle = TestCircuitHandle::new("check_substr_inclusion_poly_boolean_test.circom").unwrap();

    let max_str_len = 100;
    let max_substr_len = 20;
    let config = CircuitPaddingConfig::new().max_length("str", max_str_len).max_length("substr", max_substr_len);
    let string = "Hello World!";
    let string_hash = poseidon_bn254::pad_and_hash_string(&string, max_str_len).unwrap();
    let substring = "lo Wor";
    let substring_len = substring.len();
    let start_index = 3;
    let expected_output = 1;


    let circuit_input_signals = CircuitInputSignals::new().str_input("str", string).str_input("substr", substring).u64_input("substr_len", substring_len as u64).u64_input("start_index", start_index).fr_input("str_hash", string_hash).u64_input("expected_output", expected_output).pad(&config).unwrap();

     
    let result = circuit_handle.gen_witness(circuit_input_signals);
    assert!(result.is_ok());
}

#[test]
fn concatenation_check_test() {
    let circuit_handle = TestCircuitHandle::new("concatenation_check_test.circom").unwrap();

    let max_full_str_len = 100;
    let max_left_str_len = 70;
    let max_right_str_len = 70;
    let config = CircuitPaddingConfig::new().max_length("full_string", max_full_str_len).max_length("left", max_left_str_len).max_length("right", max_right_str_len);
    let full_string = "Hello World!";
    let left_string = "Hello ";
    let right_string = "World!";
    let left_len = left_string.len();
    let right_len = right_string.len();


    let circuit_input_signals = CircuitInputSignals::new().str_input("full_string", full_string).str_input("left", left_string).str_input("right", right_string).u64_input("left_len", left_len as u64).u64_input("right_len", right_len as u64).pad(&config).unwrap();

     
    let result = circuit_handle.gen_witness(circuit_input_signals);
    assert!(result.is_ok());
}

#[test]
fn check_are_ascii_digits_test() {
    let circuit_handle = TestCircuitHandle::new("check_are_ascii_digits_test.circom").unwrap();
    let max_input_len = 20;
    let input_arr = [48,49,50,52,55,3,0,200];
    let len = 5;
    let config = CircuitPaddingConfig::new().max_length("in", max_input_len);
    let circuit_input_signals = CircuitInputSignals::new().u64_input("len", len).bytes_input("in", &input_arr).pad(&config).unwrap();
     
    let result = circuit_handle.gen_witness(circuit_input_signals);
    assert!(result.is_ok());
}

#[test]
fn ascii_digits_to_field_test() {
    let circuit_handle = TestCircuitHandle::new("ascii_digits_to_field_test.circom").unwrap();
    let max_input_len = 20;
    let digits = [50,49,50,52,55,3,0,200]; // 21247
    let len = 5;
    let expected_output = 21247;
    let config = CircuitPaddingConfig::new().max_length("digits", max_input_len);
    let circuit_input_signals = CircuitInputSignals::new().u64_input("len", len).bytes_input("digits", &digits).u64_input("expected_output", expected_output).pad(&config).unwrap();
     
    let result = circuit_handle.gen_witness(circuit_input_signals);
    assert!(result.is_ok());
}
