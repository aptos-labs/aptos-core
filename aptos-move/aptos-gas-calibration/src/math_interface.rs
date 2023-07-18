// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//use nalgebra::DMatrix;
use std::collections::BTreeMap;

const DEFAULT_VALUE: f64 = 0.0;

/*
 * @notice: Getter function for number of rows
 * @param input: All the equations
 * @return len: Number of rows to have
 */
pub fn total_num_rows(input: Vec<BTreeMap<String, u64>>) -> usize {
    input.len()
}

/*
 * @notice: Taking the union finds the total distinct gas parameters
 * @param input: All the equations in the simplified mapping version
 * @return len: Number of columns to have
 */
pub fn total_num_of_cols(input: Vec<BTreeMap<String, u64>>) -> usize {
    let mut union_btreemap: BTreeMap<String, u64> = BTreeMap::new();
    for map in input {
        union_btreemap.extend(map);
    }
    union_btreemap.len()
}

/*
 * @notice: Creates a generic template for BTreeMaps so we can easily
 * translate the entries into indices of our vector representation
 * @param input: All the equations in the simplified mapping version
 * @return generic: A template map
 */
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

/*
 * @notice: Standardize all maps into a generic map
 * @param input: All the equations in the simplified mapping version
 * @return generic_maps: All the equations now in the standardized format
 */
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

pub fn convert_to_matrix_format(input: Vec<BTreeMap<String, u64>>) -> Vec<Vec<f64>> {
    let ncols = total_num_of_cols(input.clone());
    let mut result: Vec<Vec<f64>> = Vec::new();
    let generic_maps = convert_to_generic_map(input);
    for eq in generic_maps {
        let vec_format: Vec<f64> = eq.values().cloned().collect();
        assert_eq!(vec_format.len(), ncols);
        result.push(vec_format);
    }

    result
}
