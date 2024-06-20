// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use super::config::CircuitPaddingConfig;
use anyhow::{anyhow, bail, Result};
use ark_bn254::Fr;
use serde_json::Value;
use std::{collections::BTreeMap, marker::PhantomData};

#[derive(Debug)]
pub enum CircuitInputSignal {
    U64(u64),
    Fr(Fr),
    Frs(Vec<Fr>),
    Limbs(Vec<u64>),
    Bytes(Vec<u8>),
}

pub struct Unpadded;

#[derive(Debug)]
pub struct Padded;

#[derive(Debug)]
pub struct CircuitInputSignals<T> {
    signals: BTreeMap<String, CircuitInputSignal>,
    t: PhantomData<T>,
}

impl Default for CircuitInputSignals<Unpadded> {
    fn default() -> Self {
        Self::new()
    }
}

impl CircuitInputSignals<Unpadded> {
    pub fn new() -> Self {
        Self {
            signals: BTreeMap::new(),
            t: PhantomData,
        }
    }

    pub fn bytes_input(mut self, signal_name: &str, signal_value: &[u8]) -> Self {
        self.signals.insert(
            String::from(signal_name),
            CircuitInputSignal::Bytes(Vec::from(signal_value)),
        );
        self
    }

    pub fn byte_input(mut self, signal_name: &str, signal_value: u8) -> Self {
        self.signals.insert(
            String::from(signal_name),
            CircuitInputSignal::U64(signal_value as u64),
        );
        self
    }

    pub fn bits_input(self, signal_name: &str, signal_value: &[bool]) -> Self {
        let bytes: Vec<u8> = signal_value.iter().map(|&val| val as u8).collect();
        self.bytes_input(signal_name, bytes.as_slice())
    }

    pub fn str_input(mut self, signal_name: &str, signal_value: &str) -> Self {
        self.signals.insert(
            String::from(signal_name),
            CircuitInputSignal::Bytes(Vec::from(signal_value.as_bytes())),
        );
        self
    }

    pub fn usize_input(mut self, signal_name: &str, signal_value: usize) -> Self {
        self.signals.insert(
            String::from(signal_name),
            CircuitInputSignal::U64(signal_value as u64),
        );
        self
    }

    pub fn limbs_input(mut self, signal_name: &str, signal_value: &[u64]) -> Self {
        self.signals.insert(
            String::from(signal_name),
            CircuitInputSignal::Limbs(Vec::from(signal_value)),
        );
        self
    }

    pub fn u64_input(mut self, signal_name: &str, signal_value: u64) -> Self {
        self.signals.insert(
            String::from(signal_name),
            CircuitInputSignal::U64(signal_value),
        );
        self
    }

    pub fn frs_input(mut self, signal_name: &str, signal_value: &[Fr]) -> Self {
        self.signals.insert(
            String::from(signal_name),
            CircuitInputSignal::Frs(Vec::from(signal_value)),
        );
        self
    }

    pub fn fr_input(mut self, signal_name: &str, signal_value: Fr) -> Self {
        self.signals.insert(
            String::from(signal_name),
            CircuitInputSignal::Fr(signal_value),
        );
        self
    }

    pub fn bools_input(mut self, signal_name: &str, signal_value: &[bool]) -> Self {
        self.signals.insert(
            String::from(signal_name),
            CircuitInputSignal::Bytes(signal_value.iter().map(|b| *b as u8).collect::<Vec<u8>>()),
        );
        self
    }

    pub fn bool_input(mut self, signal_name: &str, signal_value: bool) -> Self {
        self.signals.insert(
            String::from(signal_name),
            CircuitInputSignal::U64(signal_value as u64),
        );
        self
    }

    pub fn merge(mut self, to_merge: CircuitInputSignals<Unpadded>) -> Result<Self> {
        for (key, _) in self.signals.iter() {
            if to_merge.signals.contains_key(key) {
                bail!("Cannot redefine a signal input that is already defined.")
            }
        }

        self.signals.extend(to_merge.signals);

        Ok(Self {
            signals: self.signals,
            t: PhantomData,
        })
    }

    pub fn pad(self, config: &CircuitPaddingConfig) -> Result<CircuitInputSignals<Padded>> {
        let padded_signals_vec: Result<Vec<(String, CircuitInputSignal)>> = self
            .signals
            .into_iter()
            .map(|(k, v)| {
                anyhow::Ok((String::from(&k), pad_if_needed(&k, v, &config.max_lengths)?))
            })
            .collect();

        let padded_signals: BTreeMap<String, CircuitInputSignal> =
            BTreeMap::from_iter(padded_signals_vec?);

        Ok(CircuitInputSignals {
            signals: padded_signals,
            t: PhantomData,
        })
    }
}

// padding helper functions

fn pad_if_needed(
    k: &str,
    v: CircuitInputSignal,
    global_input_max_lengths: &BTreeMap<String, usize>,
) -> Result<CircuitInputSignal, anyhow::Error> {
    Ok(match v {
        CircuitInputSignal::U64(x) => CircuitInputSignal::U64(x),
        CircuitInputSignal::Fr(x) => CircuitInputSignal::Fr(x),
        CircuitInputSignal::Frs(x) => CircuitInputSignal::Frs(x),
        CircuitInputSignal::Limbs(mut x) => {
            let zeros_needed =
                global_input_max_lengths.get(k).copied().unwrap_or(x.len()) - x.len();
            x.extend(vec![0; zeros_needed]);
            CircuitInputSignal::Limbs(x)
        },

        CircuitInputSignal::Bytes(b) => {
            CircuitInputSignal::Bytes(pad_bytes(&b, global_input_max_lengths[k])?)
        },
    })
}

fn pad_bytes(unpadded_bytes: &[u8], max_size: usize) -> Result<Vec<u8>, anyhow::Error> {
    let mut bytes = Vec::from(unpadded_bytes);

    println!("size: {}", bytes.len());
    println!("max size: {}", max_size);
    println!("{:?}", String::from_utf8_lossy(&bytes));

    if max_size < bytes.len() {
        Err(anyhow!("max_size exceeded"))
    } else {
        bytes.extend([0].repeat(max_size - bytes.len()));
        Ok(bytes)
    }
}

/// Can only serialize a CircuitInputSignals struct if padding has been added
impl CircuitInputSignals<Padded> {
    pub fn to_json_value(self) -> serde_json::Value {
        Value::from(serde_json::Map::from_iter(
            self.signals.into_iter().map(|(k, v)| (k, stringify(v))),
        ))
    }
}

fn stringify_vec<T: ToString>(v: &[T]) -> Vec<String> {
    v.iter().map(|num| num.to_string()).collect()
}

fn stringify_vec_fr(v: &[Fr]) -> Vec<String> {
    v.iter().map(fr_to_string).collect()
}

fn stringify(input: CircuitInputSignal) -> Value {
    match input {
        CircuitInputSignal::U64(x) => Value::from(x.to_string()),
        CircuitInputSignal::Fr(x) => Value::from(fr_to_string(&x)),
        CircuitInputSignal::Frs(x) => Value::from(stringify_vec_fr(&x)),
        CircuitInputSignal::Limbs(x) => Value::from(stringify_vec(&x)),
        CircuitInputSignal::Bytes(x) => Value::from(stringify_vec(&x)),
    }
}

/// Annoyingly, Fr serializes 0 to the empty string. Mitigate this here
fn fr_to_string(fr: &Fr) -> String {
    let s = fr.to_string();
    if s.is_empty() {
        String::from("0")
    } else {
        s
    }
}
