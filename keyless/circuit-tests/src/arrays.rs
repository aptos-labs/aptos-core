use aptos_keyless_common::input_processing::circuit_input_signals::CircuitInputSignals;
use aptos_keyless_common::input_processing::config::CircuitPaddingConfig;
use aptos_keyless_common::input_processing::witness_gen::witness_gen;
use crate::{TestCircuitHandle};
use itertools::*;
     
#[test]
fn array_selector_test() {
    let circuit_handle = TestCircuitHandle::new("array_selector_test.circom").unwrap();
    let output = [0,0,1,1,1,0,0,0];
    let start_index = 2;
    let end_index = 5;
    let out_len = 8;
    let config = CircuitPaddingConfig::new().max_length("expected_output", out_len);
    let circuit_input_signals = CircuitInputSignals::new().u64_input("start_index", start_index).u64_input("end_index", end_index).bytes_input("expected_output", &output).pad(&config).unwrap();
     
    let result = circuit_handle.gen_witness(circuit_input_signals);
    assert!(result.is_ok());
}

#[test]
fn array_selector_complex_test() {
    let circuit_handle = TestCircuitHandle::new("array_selector_complex_test.circom").unwrap();
    let output = [0,0,1,1,1,0,0,0];
    let start_index = 2;
    let end_index = 5;
    let out_len = 8;
    let config = CircuitPaddingConfig::new().max_length("expected_output", out_len);
    let circuit_input_signals = CircuitInputSignals::new().u64_input("start_index", start_index).u64_input("end_index", end_index).bytes_input("expected_output", &output).pad(&config).unwrap();
     
    let result = circuit_handle.gen_witness(circuit_input_signals);
    assert!(result.is_ok());
}

#[test]
fn left_array_selector_test() {
    let circuit_handle = TestCircuitHandle::new("left_array_selector_test.circom").unwrap();
    let output = [1,1,0,0,0,0,0,0];
    let index = 2;
    let out_len = 8;
    let config = CircuitPaddingConfig::new().max_length("expected_output", out_len);
    let circuit_input_signals = CircuitInputSignals::new().u64_input("index", index).bytes_input("expected_output", &output).pad(&config).unwrap();
     
    let result = circuit_handle.gen_witness(circuit_input_signals);
    assert!(result.is_ok());
}

#[test]
fn right_array_selector_test() {
    let circuit_handle = TestCircuitHandle::new("right_array_selector_test.circom").unwrap();
    let output = [0,0,0,1,1,1,1,1];
    let index = 2;
    let out_len = 8;
    let config = CircuitPaddingConfig::new().max_length("expected_output", out_len);
    let circuit_input_signals = CircuitInputSignals::new().u64_input("index", index).bytes_input("expected_output", &output).pad(&config).unwrap();
     
    let result = circuit_handle.gen_witness(circuit_input_signals);
    assert!(result.is_ok());
}

#[test]
fn single_one_array_test() {
    let circuit_handle = TestCircuitHandle::new("single_one_array_test.circom").unwrap();
    let output = [0,0,1,0,0,0,0,0];
    let index = 2;
    let out_len = 8;
    let config = CircuitPaddingConfig::new().max_length("expected_output", out_len);
    let circuit_input_signals = CircuitInputSignals::new().u64_input("index", index).bytes_input("expected_output", &output).pad(&config).unwrap();
     
    let result = circuit_handle.gen_witness(circuit_input_signals);
    assert!(result.is_ok());
}
