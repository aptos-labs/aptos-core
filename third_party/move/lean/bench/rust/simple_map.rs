// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! `aptos_std::simple_map` in Rust: an unsorted map over `Vec`.

// Keys are `u64`, not `K: PartialEq`, whose `==` needs trait evidence.

pub struct Element<V> {
    key: u64,
    value: V,
}

pub struct SimpleMap<V> {
    data: Vec<Element<V>>,
}

impl<V> SimpleMap<V> {
    pub fn new() -> Self {
        SimpleMap { data: Vec::new() }
    }

    pub fn length(&self) -> usize {
        self.data.len()
    }

    // Idiomatic: `self.data.iter().position(|element| element.key == *key)`.
    fn find(&self, key: &u64) -> Option<usize> {
        for i in 0..self.data.len() {
            if self.data[i].key == *key {
                return Some(i);
            }
        }
        None
    }

    pub fn contains_key(&self, key: &u64) -> bool {
        self.find(key).is_some()
    }

    pub fn add(&mut self, key: u64, value: V) {
        // Idiomatic: `assert!(self.find(&key).is_none())`.
        if self.find(&key).is_some() {
            std::process::abort();
        }
        self.data.push(Element { key, value });
    }

    pub fn remove(&mut self, key: &u64) -> (u64, V) {
        // Idiomatic: `expect("key not found")` here and below.
        let index = self.find(key).unwrap();
        let Element { key, value } = self.data.swap_remove(index);
        (key, value)
    }

    pub fn borrow(&self, key: &u64) -> &V {
        let index = self.find(key).unwrap();
        // Idiomatic: `&self.data[index].value`, which meets the opaque `find`'s dependency.
        let data: &[Element<V>] = &self.data;
        &data[index].value
    }

    pub fn borrow_mut(&mut self, key: &u64) -> &mut V {
        let index = self.find(key).unwrap();
        &mut self.data[index].value
    }
}
