// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

pub struct Guard {
    pub value: u32,
}

impl Drop for Guard {
    fn drop(&mut self) {}
}

pub struct Guards {
    pub values: Vec<Guard>,
}

pub struct Mixed {
    pub values: Vec<u32>,
    pub guard: Guard,
}

pub struct Generic<T> {
    pub values: Vec<T>,
}

pub struct Own {
    pub values: Vec<u32>,
}

impl Drop for Own {
    fn drop(&mut self) {}
}

pub fn guards(_value: Guards) -> u32 {
    1
}

pub fn mixed(_value: Mixed) -> u32 {
    1
}

pub fn generic(_value: Generic<Guard>) -> u32 {
    1
}

pub fn own(_value: Own) -> u32 {
    1
}

pub fn nested(_value: Vec<Vec<Guard>>) -> u32 {
    1
}

pub fn of_mixed(_value: Vec<Mixed>) -> u32 {
    1
}

pub fn optional(_value: Option<Vec<Guard>>) -> u32 {
    1
}
