// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::TestCircuitHandle;
use aptos_keyless_common::input_processing::{
    circuit_input_signals::CircuitInputSignals, config::CircuitPaddingConfig,
};
use ark_bn254::Fr;
use ark_ff::PrimeField;
use rand::{distributions::Alphanumeric, rngs::ThreadRng, Rng}; // 0.8
use std::iter::zip;

fn generate_string_bodies_input() -> String {
    let mut rng = rand::thread_rng();

    let len = 13;

    let mut s: Vec<u8> = rng
        .sample_iter(&Alphanumeric)
        .take(len)
        .collect::<String>()
        .as_bytes()
        .into();

    let num_to_replace = rng.gen_range(0, len);
    let to_replace: Vec<usize> = (0..num_to_replace).map(|_| rng.gen_range(0, len)).collect();
    let replace_with_escaped_quote: Vec<bool> =
        (0..num_to_replace).map(|_| rng.gen_bool(0.5)).collect();

    for (i, should_replace_with_escaped_quote) in zip(to_replace, replace_with_escaped_quote) {
        if should_replace_with_escaped_quote && i > 0 {
            s[i - 1] = b'\\';
            s[i] = b'"';
        } else {
            s[i] = b'"';
        }
    }

    String::from_utf8_lossy(&s).into_owned()
}

fn format_quotes_array(q: &[bool]) -> String {
    q.iter()
        .map(|b| match b {
            true => "1",
            false => "0",
        })
        .collect::<Vec<&str>>()
        .concat()
}

pub fn calc_string_bodies(s: &str) -> Vec<bool> {
    let bytes = s.as_bytes();
    let mut string_bodies = vec![false; s.len()];
    let mut quotes = vec![false; s.len()];
    let mut quote_parity = vec![false; s.len()];

    quotes[0] = bytes[0] == b'"';
    quote_parity[0] = bytes[0] == b'"';
    for i in 1..bytes.len() {
        let mut prev_is_odd_backslash = false;
        for j in (0..i).rev() {
            if bytes[j] != b'\\' {
                break;
            }
            println!("{}: {}", j, bytes[j]);
            prev_is_odd_backslash = !prev_is_odd_backslash;
        }
        quotes[i] = bytes[i] == b'"' && !prev_is_odd_backslash;
        quote_parity[i] = if quotes[i] {
            !quote_parity[i - 1]
        } else {
            quote_parity[i - 1]
        };
    }

    string_bodies[0] = false;
    for i in 1..bytes.len() {
        string_bodies[i] = quote_parity[i] && quote_parity[i - 1];
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
    let quotes_b: Vec<bool> = quotes.iter().map(|b| b == &1u8).collect();

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
    let quotes_b: Vec<bool> = quotes.iter().map(|b| b == &1u8).collect();

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

    for _iter in 0..128 {
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

#[test]
fn string_bodies_test_prefix_quotes() {
    let circuit_handle = TestCircuitHandle::new("misc/string_bodies_test.circom").unwrap();

    for i in 0..13 {
        let mut bytes = vec![b'a'; 13];
        for byte in bytes.iter_mut().take(i) {
            *byte = b'"';
        }
        let s = String::from_utf8_lossy(&bytes);

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

#[test]
fn string_bodies_test_zjma() {
    let circuit_handle = TestCircuitHandle::new("misc/string_bodies_test.circom").unwrap();

    let s = "\"abc\\\\\"";
    let quotes = calc_string_bodies(s);

    let config = CircuitPaddingConfig::new()
        .max_length("in", 13)
        .max_length("out", 13);

    let circuit_input_signals = CircuitInputSignals::new()
        .str_input("in", s)
        .bools_input("out", &quotes)
        .pad(&config)
        .unwrap();

    let result = circuit_handle.gen_witness(circuit_input_signals);
    println!("{:?}", result);
    assert!(result.is_ok());
}

#[test]
fn calculate_total_test() {
    let circuit_handle = TestCircuitHandle::new("misc/calculate_total_test.circom").unwrap();

    let mut rng = rand::thread_rng();

    for _i in 0..256 {
        let nums: Vec<Fr> = (0..10).map(|_| Fr::from(rng.gen::<u64>())).collect();

        let sum: Fr = nums.iter().sum();

        let config = CircuitPaddingConfig::new().max_length("nums", 10);

        let circuit_input_signals = CircuitInputSignals::new()
            .frs_input("nums", &nums)
            .fr_input("sum", sum)
            .pad(&config)
            .unwrap();

        let result = circuit_handle.gen_witness(circuit_input_signals);
        println!("{:?}", result);
        assert!(result.is_ok());
    }
}

#[test]
fn assert_equal_if_true_test() {
    fn rand_fr(rng: &mut ThreadRng) -> Fr {
        let bytes: [u8; 32] = rng.gen();
        Fr::from_le_bytes_mod_order(&bytes)
    }

    let circuit_handle = TestCircuitHandle::new("misc/assert_equal_if_true_test.circom").unwrap();

    let mut rng = rand::thread_rng();

    for _i in 0..256 {
        let (nums, are_equal) = if rng.gen_bool(0.5) {
            let nums: Vec<Fr> = (0..2).map(|_| rand_fr(&mut rng)).collect();

            let mut are_equal = nums[0] == nums[1];

            if rng.gen_bool(0.5) {
                are_equal = true;
            }

            (nums, are_equal)
        } else {
            let num = rand_fr(&mut rng);
            let nums: Vec<Fr> = vec![num, num];

            (nums, true)
        };

        let config = CircuitPaddingConfig::new().max_length("in", 2);

        let circuit_input_signals = CircuitInputSignals::new()
            .frs_input("in", &nums)
            .bool_input("bool", are_equal)
            .pad(&config)
            .unwrap();

        let result = circuit_handle.gen_witness(circuit_input_signals);
        println!("{:?}", result);
        if are_equal == (nums[0] == nums[1]) {
            assert!(result.is_ok());
        } else {
            assert!(result.is_err());
        }
    }
}

#[test]
fn email_verified_check_test() {
    let circuit_handle = TestCircuitHandle::new("misc/email_verified_check_test.circom").unwrap();

    let testcases = [
        ("email_verified", "true", "email", true, true),
        // Note that this template doesn't actually check that ev_name is exactly equal to
        // "email_verified". It only checks that it starts with this string. I believe that this
        // is not an issue because ParseEmailVerifiedField enforces that ev_name has len == 14.
        ("email_verified000", "true", "email", true, true),
        ("email_verified", "false", "email", true, false),
        ("email_verified", "true", "sub", false, true),
        ("email_verified", "false", "sub", false, true),
    ];

    for t in testcases {
        let ev_name = t.0;
        let ev_value = t.1;
        let uid_name = t.2;
        let expected_uid_is_email = t.3;
        let test_should_pass = t.4;

        let config = CircuitPaddingConfig::new()
            .max_length("ev_name", 20)
            .max_length("ev_value", 10)
            .max_length("uid_name", 30);

        let circuit_input_signals = CircuitInputSignals::new()
            .str_input("ev_name", ev_name)
            .str_input("ev_value", ev_value)
            .str_input("uid_name", uid_name)
            .usize_input("ev_value_len", ev_value.len())
            .usize_input("uid_name_len", uid_name.len())
            .bool_input("uid_is_email", expected_uid_is_email)
            .pad(&config)
            .unwrap();

        let result = circuit_handle.gen_witness(circuit_input_signals);
        println!("{:?}", result);
        if test_should_pass {
            assert!(result.is_ok());
        } else {
            assert!(result.is_err());
        }
    }
}
