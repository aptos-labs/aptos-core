
use crate::TestCircuitHandle;
use aptos_keyless_common::input_processing::{
    circuit_input_signals::CircuitInputSignals, config::CircuitPaddingConfig,
};
use aptos_crypto::poseidon_bn254;

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
