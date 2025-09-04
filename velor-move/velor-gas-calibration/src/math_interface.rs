// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use std::collections::BTreeMap;

const DEFAULT_VALUE: f64 = 0.0;

/// Getter function for number of rows
///
/// ### Arguments
///
/// * `input` - All the equations
pub fn total_num_rows(input: Vec<BTreeMap<String, u64>>) -> usize {
    input.len()
}

/// Taking the union finds the total distinct gas parameters
///
/// ### Arguments
///
/// * `input` - All the equations in the simplified mapping version
pub fn total_num_of_cols(input: Vec<BTreeMap<String, u64>>) -> usize {
    let mut union_btreemap: BTreeMap<String, u64> = BTreeMap::new();
    for map in input {
        union_btreemap.extend(map);
    }
    union_btreemap.len()
}

/// Creates a generic template for BTreeMaps so we can easily
/// translate the entries into indices of our vector representation
///
/// ### Arguments
///
/// * `input` - All the equations in the simplified mapping version
pub fn generic_map(input: Vec<BTreeMap<String, u64>>) -> BTreeMap<String, f64> {
    let mut union_btreemap: BTreeMap<String, u64> = BTreeMap::new();
    for map in input {
        union_btreemap.extend(map);
    }

    // sort BTree lexicographically
    let sorted_map: BTreeMap<String, u64> = union_btreemap.into_iter().collect();

    let keys: Vec<String> = sorted_map.keys().map(|key| key.to_string()).collect();
    let generic: BTreeMap<String, f64> = keys.into_iter().map(|key| (key, DEFAULT_VALUE)).collect();
    generic
}

/// Standardize all maps into a generic map
///
/// ### Arguments
///
/// * `input` - All the equations in the simplified mapping version
pub fn convert_to_generic_map(input: Vec<BTreeMap<String, u64>>) -> Vec<BTreeMap<String, f64>> {
    let mut generic_maps: Vec<BTreeMap<String, f64>> = Vec::new();
    for map in &input {
        let mut generic = generic_map(input.clone());
        for (key, value) in map.iter() {
            generic
                .entry(key.to_string())
                .and_modify(|v| *v = *value as f64);
        }
        generic_maps.push(generic);
    }
    generic_maps
}

/// Transform into standardized mapping before converting to a vector
///
/// ### Arguments
///
/// * `input` - All the equations in the simplified mapping version
pub fn convert_to_matrix_format(input: Vec<BTreeMap<String, u64>>) -> Vec<Vec<f64>> {
    let ncols = total_num_of_cols(input.clone());
    let mut result: Vec<Vec<f64>> = Vec::new();
    let generic_maps = convert_to_generic_map(input);
    //println!("KEYS {:?}\n", generic_maps);
    for eq in generic_maps {
        let vec_format: Vec<f64> = eq.values().cloned().collect();
        assert_eq!(vec_format.len(), ncols);
        result.push(vec_format);
    }

    result
}
