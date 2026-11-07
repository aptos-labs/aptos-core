// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

// Cases the `no_std_hash_map` lint must flag, and ones it must not.
#![allow(dead_code, unused_variables, unused_imports)]

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

type Alias<V> = HashMap<u32, V>;

struct Holder {
    map: HashMap<u32, u64>,
    set: HashSet<String>,
    nested: Vec<Option<HashMap<u8, u8>>>,
    // Not flagged: ordered collections are fine.
    ordered: BTreeMap<u32, u64>,
    ordered_set: BTreeSet<u32>,
}

fn takes(m: &HashMap<u32, u64>) -> HashSet<u32> {
    HashSet::new()
}

fn constructors() {
    let a = HashMap::<u32, u32>::new();
    let b: HashMap<u32, u32> = Default::default();
    let c = HashSet::<u32>::with_capacity(4);
    let d = <HashMap<u32, u32>>::default();
    // Inferred only through use.
    let mut e = Default::default();
    takes(&e);
    // Through an alias.
    let f: Alias<u8> = Alias::new();
    // Turbofish on collect.
    let g = (0..3).map(|i| (i, i)).collect::<HashMap<_, _>>();
    // Not flagged.
    let h: BTreeMap<u32, u32> = BTreeMap::new();
    let i = BTreeSet::<u32>::new();
}

#[cfg_attr(dylint_lib = "aptos_lints", allow(no_std_hash_map))]
fn exempted() {
    let m: HashMap<u32, u32> = HashMap::new();
}

fn main() {}
