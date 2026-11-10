// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// False twins of `bench/rust/simple_map`: each claim below fails.

pub struct Element<V> {
    key: u64,
    value: V,
}

pub struct SimpleMap<V> {
    data: Vec<Element<V>>,
}

impl<V> SimpleMap<V> {
    fn find(&self, key: &u64) -> Option<usize> {
        for i in 0..self.data.len() {
            if self.data[i].key == *key {
                return Some(i);
            }
        }
        None
    }

    pub fn first(&self, key: &u64) -> Option<usize> {
        for i in 0..self.data.len() {
            if self.data[i].key == *key {
                return Some(i);
            }
        }
        None
    }

    pub fn add_checked(&mut self, key: u64, value: V) {
        if self.find(&key).is_some() {
            std::process::abort();
        }
        self.data.push(Element { key, value });
    }

    pub fn add_unchecked(&mut self, key: u64, value: V) {
        self.data.push(Element { key, value });
    }

    pub fn remove(&mut self, key: &u64) -> (u64, V) {
        let index = self.find(key).unwrap();
        let Element { key, value } = self.data.swap_remove(index);
        (key, value)
    }

    pub fn borrow(&self, key: &u64) -> &V {
        let index = self.find(key).unwrap();
        let data: &[Element<V>] = &self.data;
        &data[index].value
    }
}
