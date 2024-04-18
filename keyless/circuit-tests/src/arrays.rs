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
    let in_len = array.len();
    for index in 0..in_len {
        let output = array[index];
        let config = CircuitPaddingConfig::new().max_length("array", in_len);
        let circuit_input_signals = CircuitInputSignals::new().u64_input("index", index as u64).bytes_input("array", &array).u64_input("expected_output", output as u64).pad(&config).unwrap();
     
        let result = circuit_handle.gen_witness(circuit_input_signals);
        assert!(result.is_ok());
    }
}

#[test]
fn select_array_value_large_test() {
    let circuit_handle = TestCircuitHandle::new("select_array_value_large_test.circom").unwrap();
    let mut input = Vec::new();
    let mut i: u64 = 0;
    for _ in 0..2000 {
        input.push((i%256) as u8);
        i += 1;
    }
    let index = 1567;
    let in_len = input.len();
    let output = input[index];
    let config = CircuitPaddingConfig::new().max_length("array", in_len);
    let circuit_input_signals = CircuitInputSignals::new().u64_input("index", index as u64).bytes_input("array", &input).u64_input("expected_output", output as u64).pad(&config).unwrap();
     
    let result = circuit_handle.gen_witness(circuit_input_signals);
    assert!(result.is_ok());
}

#[test]
fn select_array_value_small_test() {
    let circuit_handle = TestCircuitHandle::new("select_array_value_small_test.circom").unwrap();
    let array = [42];
    let index = 0;
    let in_len = array.len();
    let output = array[index];
    let config = CircuitPaddingConfig::new().max_length("array", in_len);
    let circuit_input_signals = CircuitInputSignals::new().u64_input("index", index as u64).bytes_input("array", &array).u64_input("expected_output", output as u64).pad(&config).unwrap();
     
    let result = circuit_handle.gen_witness(circuit_input_signals);
    assert!(result.is_ok());
}

#[test]
#[should_panic]
fn select_array_value_test_wrong_index() {
    let circuit_handle = TestCircuitHandle::new("select_array_value_test.circom").unwrap();
    let out_len = 8;
    let index = 8;
    let output = [4,6,1,8,9,4,2,3];
    let config = CircuitPaddingConfig::new().max_length("expected_output", out_len as usize);
    let circuit_input_signals = CircuitInputSignals::new().u64_input("index", index as u64).bytes_input("expected_output", &output).pad(&config).unwrap();
     
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
fn array_selector_test() {
    let circuit_handle = TestCircuitHandle::new("array_selector_test.circom").unwrap();

    let start_index = 0;

    for end_index in 1..=8 {

        let mut expected_out = [0u8; 8];
        for i in 0..end_index {
            expected_out[i] = 1;
        }

        let config = CircuitPaddingConfig::new()
            .max_length("expected_out", 8);

        let circuit_input_signals = CircuitInputSignals::new()
            .usize_input("start_index", start_index)
            .usize_input("end_index", end_index)
            .bytes_input("expected_out", &expected_out)
            .pad(&config)
            .unwrap();


        let result = circuit_handle.gen_witness(circuit_input_signals);

        println!("result={:?}", result);
        assert!(result.is_ok());

    }

}


#[test]
fn check_substr_inclusion_poly_test() {
    let circuit_handle = TestCircuitHandle::new("check_substr_inclusion_poly_test.circom").unwrap();

    let test_str : &'static [u8] = &[4u8, 233, 24, 159, 105, 83, 145, 69, 245, 99, 150, 28, 197, 219, 186, 204, 47, 219, 5, 139, 89, 15, 216, 169, 206, 145, 224, 32, 59, 0, 178, 44, 116, 149, 61, 64, 149, 134, 204, 103, 18, 57, 87, 168, 144, 26, 173, 48, 219, 125, 64, 211, 131, 159, 76, 29, 154, 118, 163, 18, 38, 24, 44, 191, 196, 36, 240, 250, 82, 176, 94, 86, 202, 67, 142, 19, 115, 237, 104, 190, 28, 122, 44, 252, 139, 106, 125, 145, 135, 1, 181, 127, 0, 242, 187, 80, 208, 51, 22, 1, 194, 159, 218, 16, 33, 113, 220, 214, 209, 168, 195, 83, 177, 149, 74, 20, 7, 28, 124, 175, 212, 240, 55, 96, 155, 163, 158, 94, 64, 141, 154, 111, 89, 219, 90, 16, 142, 139, 215, 124, 141, 19, 94, 73, 24, 213, 204, 15, 221, 86, 52, 132, 246, 58, 133, 94, 193, 36, 12, 232, 37, 209, 171, 118, 85, 13, 154, 180, 124, 188, 81, 235, 254, 114, 114, 101, 75, 161, 208, 227, 71, 22, 48, 204, 128, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 5, 192];
    let str_hash = poseidon_bn254::pad_and_hash_bytes_with_len(test_str, 256).unwrap();
    let substr : &'static [u8] = &[0u8, 0, 0, 0, 0, 0, 5, 192];
    let start_index = 248;


    let config = CircuitPaddingConfig::new()
        .max_length("str", 256)
        .max_length("substr", 8);

    let circuit_input_signals = CircuitInputSignals::new()
        .bytes_input("str", test_str)
        .fr_input("str_hash", str_hash)
        .bytes_input("substr", substr)
        .usize_input("substr_len", substr.len())
        .usize_input("start_index", start_index)
        .pad(&config)
        .unwrap();


    let result = circuit_handle.gen_witness(circuit_input_signals);

    println!("result={:?}", result);
    assert!(result.is_ok());
}

#[test]
fn check_substr_inclusion_poly_boolean_test() {
    let circuit_handle = TestCircuitHandle::new("check_substr_inclusion_poly_boolean_test.circom").unwrap();

    let test_str : &'static [u8] = &[4u8, 233, 24, 159, 105, 83, 145, 69, 245, 99, 150, 28, 197, 219, 186, 204, 47, 219, 5, 139, 89, 15, 216, 169, 206, 145, 224, 32, 59, 0, 178, 44, 116, 149, 61, 64, 149, 134, 204, 103, 18, 57, 87, 168, 144, 26, 173, 48, 219, 125, 64, 211, 131, 159, 76, 29, 154, 118, 163, 18, 38, 24, 44, 191, 196, 36, 240, 250, 82, 176, 94, 86, 202, 67, 142, 19, 115, 237, 104, 190, 28, 122, 44, 252, 139, 106, 125, 145, 135, 1, 181, 127, 0, 242, 187, 80, 208, 51, 22, 1, 194, 159, 218, 16, 33, 113, 220, 214, 209, 168, 195, 83, 177, 149, 74, 20, 7, 28, 124, 175, 212, 240, 55, 96, 155, 163, 158, 94, 64, 141, 154, 111, 89, 219, 90, 16, 142, 139, 215, 124, 141, 19, 94, 73, 24, 213, 204, 15, 221, 86, 52, 132, 246, 58, 133, 94, 193, 36, 12, 232, 37, 209, 171, 118, 85, 13, 154, 180, 124, 188, 81, 235, 254, 114, 114, 101, 75, 161, 208, 227, 71, 22, 48, 204, 128, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 5, 192];
    let str_hash = poseidon_bn254::pad_and_hash_bytes_with_len(test_str, 256).unwrap();
    let substr : &'static [u8] = &[0u8, 0, 0, 0, 0, 0, 5, 192];
    let start_index = 248;


    let config = CircuitPaddingConfig::new()
        .max_length("str", 256)
        .max_length("substr", 8);

    let circuit_input_signals = CircuitInputSignals::new()
        .bytes_input("str", test_str)
        .fr_input("str_hash", str_hash)
        .bytes_input("substr", substr)
        .usize_input("substr_len", substr.len())
        .usize_input("start_index", start_index)
        .bool_input("check_passes", true)
        .pad(&config)
        .unwrap();


    let result = circuit_handle.gen_witness(circuit_input_signals);

    println!("result={:?}", result);
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
