
// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::TestCircuitHandle;
use aptos_keyless_common::input_processing::{
    circuit_input_signals::{CircuitInputSignals, Padded}, config::CircuitPaddingConfig,
};


//fn generate_string_bodies_input() -> String {
//}

pub fn calc_string_bodies(s: &str) -> Vec<bool> {
    let bytes = s.as_bytes();
    let mut string_bodies = vec![false; s.len()];
    let mut quotes = vec![false; s.len()];


    string_bodies[0] = false;
    string_bodies[1] = (bytes[0] == b'"');

    for i in 2..bytes.len() {
        // should we start a string body?
        if string_bodies[i-2] == false && bytes[i-1] == b'"' && bytes[i-2] != b'\\' {
            string_bodies[i] = true;
        // should we end a string body?
        } else if string_bodies[i-1] == true && bytes[i] == b'"' && bytes[i-1] != b'\\' {
            string_bodies[i] = false;
        } else {
            string_bodies[i] = string_bodies[i-1];
        }
        
    }

    string_bodies
}

#[test]
fn is_whitespace_test() {
    let circuit_handle = TestCircuitHandle::new("misc/is_whitespace_test.circom").unwrap();


    for c in 0u8..=127u8 {
        let config = CircuitPaddingConfig::new();

        let circuit_input_signals = CircuitInputSignals::new()
            .byte_input("char", c)
            .bool_input("result", (c as char).is_whitespace())
            .pad(&config)
            .unwrap();

        let result = circuit_handle.gen_witness(circuit_input_signals);
        println!("{}: {:?}", c, result);
        assert!(result.is_ok());
    }
}



#[test]
fn string_bodies_test() {
    let circuit_handle = TestCircuitHandle::new("misc/string_bodies_test.circom").unwrap();

    let s = "\"123\" 456 \"7\"";
    let quotes = &[0u8, 1, 1, 1, 0, 0, 0, 0, 0, 0, 0, 1, 0];
    let quotes_b : Vec<bool> = 
        quotes
        .iter()
        .map(|b| b == &1u8)
        .collect();

    assert_eq!(quotes_b, calc_string_bodies(s));



    let config = CircuitPaddingConfig::new()
        .max_length("in", 13)
        .max_length("out", 13);

    let circuit_input_signals = CircuitInputSignals::new()
        .str_input("in", s)
        .bytes_input("out", quotes)
        .pad(&config)
        .unwrap();

    let result = circuit_handle.gen_witness(circuit_input_signals);
    println!("{:?}", result);
    assert!(result.is_ok());
}


#[test]
fn string_bodies_test_2() {
    let circuit_handle = TestCircuitHandle::new("misc/string_bodies_test.circom").unwrap();

    let s = "\"12\\\"456\" \"7\"";
    let quotes = &[0u8, 1, 1, 1, 1, 1, 1, 1, 0, 0, 0, 1, 0];
    let quotes_b : Vec<bool> = 
        quotes
        .iter()
        .map(|b| b == &1u8)
        .collect();

    assert_eq!(quotes_b, calc_string_bodies(s));


    let config = CircuitPaddingConfig::new()
        .max_length("in", 13)
        .max_length("out", 13);

    let circuit_input_signals = CircuitInputSignals::new()
        .str_input("in", s)
        .bytes_input("out", quotes)
        .pad(&config)
        .unwrap();

    let result = circuit_handle.gen_witness(circuit_input_signals);
    println!("{:?}", result);
    assert!(result.is_ok());
}
