
// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::TestCircuitHandle;
use aptos_keyless_common::input_processing::{
    circuit_input_signals::{CircuitInputSignals, Padded}, config::CircuitPaddingConfig,
};
use std::iter::zip;
use rand::{distributions::Alphanumeric, Rng}; // 0.8



fn generate_string_bodies_input() -> String {
    let mut rng = rand::thread_rng();

    let mut s : Vec<u8> = rng
        .sample_iter(&Alphanumeric)
        .take(13)
        .collect::<String>()
        .as_bytes()
        .into();

     let num_to_replace = rng.gen_range(0,13);
     let to_replace : Vec<usize> = (0..num_to_replace).map(|_| rng.gen_range(0, 13)).collect();
     let replace_with_escaped_quote : Vec<bool> = (0..num_to_replace).map(|_| rng.gen_bool(0.5)).collect();

     for (i,should_replace_with_escaped_quote) in zip(to_replace, replace_with_escaped_quote) {
         if should_replace_with_escaped_quote && i > 0 {
             s[i-1] = b'\\';
             s[i] = b'"';
         } else {
             s[i] = b'"';
         }
     }

     String::from_utf8_lossy(&s).into_owned()
}

fn format_quotes_array(q: &[bool]) -> String {
    q.iter()
     .map(|b| match b { true => "1", false => "0" })
     .collect::<Vec<&str>>()
     .concat()
}

pub fn calc_string_bodies(s: &str) -> Vec<bool> {
    let bytes = s.as_bytes();
    let mut string_bodies = vec![false; s.len()];
    let mut quotes = vec![false; s.len()];
    let mut quote_parity = vec![false; s.len()];

    quotes[0] = (bytes[0] == b'"');
    quote_parity[0] = (bytes[0] == b'"');
    for i in 1..bytes.len() {
        quotes[i] = (bytes[i] == b'"' && bytes[i-1] != b'\\');
        quote_parity[i] = if  quotes[i] { !quote_parity[i-1] } else { quote_parity[i-1] };
    }

    string_bodies[0] = false;
    for i in 1..bytes.len() {
        string_bodies[i] = quote_parity[i] && quote_parity[i-1];
    }

    println!("string       : {}", s);
    println!("quote_parity : {}", format_quotes_array(&quote_parity));
    println!("string_bodies: {}", format_quotes_array(&string_bodies));

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

#[test]
fn string_bodies_test_random() {
    let circuit_handle = TestCircuitHandle::new("misc/string_bodies_test.circom").unwrap();


    for iter in 0..128 { 
        let s = generate_string_bodies_input();
        let quotes = calc_string_bodies(&s);

        let config = CircuitPaddingConfig::new()
            .max_length("in", 13)
            .max_length("out", 13);

        let circuit_input_signals = CircuitInputSignals::new()
            .str_input("in", &s)
            .bools_input("out", &quotes)
            .pad(&config)
            .unwrap();

        let result = circuit_handle.gen_witness(circuit_input_signals);
        println!("{:?}", result);
        assert!(result.is_ok());
    }
}

