// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    account::{AddressKind, NamedAddressKind},
    executor::tracing::ResourceWrite,
    prep::{canvas::BasicInput, ident::DatatypeIdent},
    state::{PersistedDatatypeIdent, PersistedObjectBucket, PersistedObjectState},
};
use anyhow::Result;
use move_core_types::{
    ability::AbilitySet,
    account_address::AccountAddress,
    int256::{I256, U256},
    language_storage::TypeTag as VmTypeTag,
    value::MoveValue,
};
use rand::{rngs::StdRng, seq::SliceRandom, Rng, SeedableRng};
use std::collections::{BTreeMap, BTreeSet};

// Probabilities and configurations
const GEN_PROB: u8 = 50;
const MUT_PROB: u8 = 50;
const TOTAL_PROB: u8 = GEN_PROB + MUT_PROB;

const GEN_INT_PROB_MIN: u8 = 10;
const GEN_INT_PROB_MAX: u8 = 10;
const GEN_INT_PROB_ANY: u8 = 80;
const GEN_INT_PROB_TOTAL: u8 = GEN_INT_PROB_MIN + GEN_INT_PROB_MAX + GEN_INT_PROB_ANY;

const MUT_INT_PROB_ADD_1: u8 = 20;
const MUT_INT_PROB_SUB_1: u8 = 20;
const MUT_INT_PROB_MUL_2: u8 = 20;
const MUT_INT_PROB_DIV_2: u8 = 20;
const MUT_INT_PROB_FLIP_BITS: u8 = 20;
const MUT_INT_PROB_TOTAL: u8 = MUT_INT_PROB_ADD_1
    + MUT_INT_PROB_SUB_1
    + MUT_INT_PROB_MUL_2
    + MUT_INT_PROB_DIV_2
    + MUT_INT_PROB_FLIP_BITS;

const GEN_VEC_SIZE_MAX: u8 = 8;

const MUT_VEC_PROB_ADD_ELEMENT: u8 = 20;
const MUT_VEC_PROB_DEL_ELEMENT: u8 = 20;
const MUT_VEC_PROB_MUT_ELEMENT: u8 = 25;
const MUT_VEC_PROB_DUP_ELEMENT: u8 = 10;
const MUT_VEC_PROB_SWAP_ELEMENTS: u8 = 10;
const MUT_VEC_PROB_REORDER: u8 = 15;
const MUT_VEC_PROB_TOTAL: u8 = MUT_VEC_PROB_ADD_ELEMENT
    + MUT_VEC_PROB_DEL_ELEMENT
    + MUT_VEC_PROB_MUT_ELEMENT
    + MUT_VEC_PROB_DUP_ELEMENT
    + MUT_VEC_PROB_SWAP_ELEMENTS
    + MUT_VEC_PROB_REORDER;

const GEN_ADDR_PROB_NAME_PRIMARY: u8 = 40;
const GEN_ADDR_PROB_NAME_DEPENDENCY: u8 = 10;
const GEN_ADDR_PROB_NAME_FRAMEWORK: u8 = 5;
const GEN_ADDR_PROB_USER: u8 = 45;
const GEN_ADDR_PROB_TOTAL: u8 = GEN_ADDR_PROB_NAME_PRIMARY
    + GEN_ADDR_PROB_NAME_DEPENDENCY
    + GEN_ADDR_PROB_NAME_FRAMEWORK
    + GEN_ADDR_PROB_USER;
const GEN_ADDR_PROB_OBJECT: u8 = 25;

const GEN_STR_MAX_LEN: usize = 32;

const GEN_STR_PROB_DICT: u8 = 30;
const GEN_STR_PROB_EMPTY: u8 = 20;
const GEN_STR_PROB_RANDOM: u8 = 50;
const GEN_STR_PROB_TOTAL: u8 = GEN_STR_PROB_DICT + GEN_STR_PROB_EMPTY + GEN_STR_PROB_RANDOM;

const MUT_STR_PROB_ADD: u8 = 25;
const MUT_STR_PROB_DEL: u8 = 20;
const MUT_STR_PROB_CHANGE: u8 = 20;
const MUT_STR_PROB_DICT: u8 = 20;
const MUT_STR_PROB_SPLICE: u8 = 15;
const MUT_STR_PROB_TOTAL: u8 = MUT_STR_PROB_ADD
    + MUT_STR_PROB_DEL
    + MUT_STR_PROB_CHANGE
    + MUT_STR_PROB_DICT
    + MUT_STR_PROB_SPLICE;

const DEFAULT_STRING_DICTIONARY: &[&str] = &[
    "",
    "test",
    "hello",
    "admin",
    "user",
    "token",
    "pool",
    "coin",
    "apt",
    "usdc",
    "usdt",
    "btc",
    "eth",
    "0",
    "1",
    "true",
    "false",
    "name",
    "symbol",
    "description",
    "uri",
    "metadata",
];

const MUT_TYPE_ARG_PROB: u8 = 30;

macro_rules! create_int {
    ($s:expr, $t:ty) => {{
        let x = $s.rng.gen_range(0, GEN_INT_PROB_TOTAL);
        if x < GEN_INT_PROB_MIN {
            <$t>::MIN
        } else if x < GEN_INT_PROB_MIN + GEN_INT_PROB_MAX {
            <$t>::MAX
        } else {
            $s.rng.r#gen()
        }
    }};
    ($s:expr, $min:expr, $max:expr, $rand:expr) => {{
        let x = $s.rng.gen_range(0, GEN_INT_PROB_TOTAL);
        if x < GEN_INT_PROB_MIN {
            $min
        } else if x < GEN_INT_PROB_MIN + GEN_INT_PROB_MAX {
            $max
        } else {
            $rand
        }
    }};
}

macro_rules! mutate_int {
    ($s:expr, $v:expr) => {{
        let x = $s.rng.gen_range(0, MUT_INT_PROB_TOTAL);
        if x < MUT_INT_PROB_ADD_1 {
            $v.wrapping_add(1)
        } else if x < MUT_INT_PROB_ADD_1 + MUT_INT_PROB_SUB_1 {
            $v.wrapping_sub(1)
        } else if x < MUT_INT_PROB_ADD_1 + MUT_INT_PROB_SUB_1 + MUT_INT_PROB_MUL_2 {
            $v.wrapping_mul(2)
        } else if x < MUT_INT_PROB_ADD_1
            + MUT_INT_PROB_SUB_1
            + MUT_INT_PROB_MUL_2
            + MUT_INT_PROB_DIV_2
        {
            $v.wrapping_div(2)
        } else {
            !$v
        }
    }};
}

macro_rules! mutate_int256 {
    ($s:expr, $t:ty, $v:expr) => {{
        let x = $s.rng.gen_range(0, MUT_INT_PROB_TOTAL);
        if x < MUT_INT_PROB_ADD_1 {
            <$t>::checked_add($v, <$t>::from(1u8)).unwrap_or(<$t>::ZERO)
        } else if x < MUT_INT_PROB_ADD_1 + MUT_INT_PROB_SUB_1 {
            <$t>::checked_sub($v, <$t>::from(1u8)).unwrap_or(<$t>::ZERO)
        } else if x < MUT_INT_PROB_ADD_1 + MUT_INT_PROB_SUB_1 + MUT_INT_PROB_MUL_2 {
            <$t>::checked_mul($v, <$t>::from(2u8)).unwrap_or(<$t>::ZERO)
        } else if x < MUT_INT_PROB_ADD_1
            + MUT_INT_PROB_SUB_1
            + MUT_INT_PROB_MUL_2
            + MUT_INT_PROB_DIV_2
        {
            <$t>::checked_div($v, <$t>::from(2u8)).unwrap_or(<$t>::ZERO)
        } else {
            <$t>::ZERO
        }
    }};
}

macro_rules! mutate_int_with_edges {
    ($s:expr, $v:expr, $t:ty) => {{
        match $s.rng.gen_range(0u8, 100) {
            0..=54 => mutate_int!($s, $v),
            55..=69 => <$t>::default(),
            70..=79 => <$t>::MIN,
            80..=89 => <$t>::MAX,
            _ => {
                let delta: $t = $s.rng.gen_range(1u8, 17u8) as $t;
                if $s.random_percent() < 50 {
                    $v.wrapping_add(delta)
                } else {
                    $v.wrapping_sub(delta)
                }
            },
        }
    }};
}

/// A pool of concrete VM TypeTag values indexed by their AbilitySet
#[derive(Clone)]
pub struct TypePool {
    /// All types in the pool, each paired with its abilities
    entries: Vec<(VmTypeTag, AbilitySet)>,
}

impl TypePool {
    /// Create a new empty type pool
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    /// Add a type with its abilities to the pool
    pub fn add(&mut self, ty: VmTypeTag, abilities: AbilitySet) {
        self.entries.push((ty, abilities));
    }

    /// Return all types whose abilities are a superset of the given constraint
    pub fn candidates_for(&self, constraint: AbilitySet) -> Vec<&VmTypeTag> {
        self.entries
            .iter()
            .filter(|(_, abilities)| constraint.is_subset(*abilities))
            .map(|(ty, _)| ty)
            .collect()
    }

    pub fn can_satisfy_all(&self, constraints: &[AbilitySet]) -> bool {
        constraints
            .iter()
            .all(|constraint| !self.candidates_for(*constraint).is_empty())
    }
}

/// Input generator and mutator
pub struct Mutator {
    // random number generator
    rng: StdRng,

    // dictionaries
    dict_address: BTreeMap<AddressKind, BTreeSet<AccountAddress>>,
    dict_string: Vec<String>,

    // type pool for generic type argument generation
    type_pool: TypePool,

    // object tracking: addresses confirmed as objects (have ObjectGroup)
    object_addresses: BTreeSet<AccountAddress>,

    // resource type -> set of object addresses where that resource exists
    // keyed by DatatypeIdent only (ignoring type_args) because converting
    // runtime VmTypeTag to TypeBase requires ability information that is
    // unavailable from write sets. matching by ident alone is sufficient
    // for fuzzing: a mismatched type arg just triggers an abort, which is
    // strictly better than the previous AccountAddress::ZERO.
    dict_object: BTreeMap<DatatypeIdent, BTreeSet<AccountAddress>>,
}

impl Mutator {
    /// Create a new random mutator
    pub fn new(
        seed: u64,
        dict_address: BTreeMap<AddressKind, BTreeSet<AccountAddress>>,
        type_pool: TypePool,
        dict_string: Vec<String>,
    ) -> Mutator {
        let dict_string = if dict_string.is_empty() {
            DEFAULT_STRING_DICTIONARY
                .iter()
                .map(|s| s.to_string())
                .collect()
        } else {
            dict_string
        };
        Self {
            rng: StdRng::seed_from_u64(seed),
            dict_address,
            dict_string,
            type_pool,
            object_addresses: BTreeSet::new(),
            dict_object: BTreeMap::new(),
        }
    }

    /// Decide whether to generate a new value or mutate an existing one
    pub fn should_mutate(&mut self, num_seeds: usize) -> Option<usize> {
        if num_seeds == 0 {
            return None;
        }

        let x = self.rng.gen_range(0, TOTAL_PROB);
        if x < GEN_PROB {
            return None;
        }

        let index = self.rng.gen_range(0, num_seeds);
        Some(index)
    }

    /// Randomly generate a Move value based on a basic input type
    pub fn random_value(&mut self, ty: &BasicInput) -> MoveValue {
        match ty {
            BasicInput::Bool => MoveValue::Bool(self.rng.r#gen()),
            BasicInput::U8 => MoveValue::U8(create_int!(self, u8)),
            BasicInput::I8 => MoveValue::I8(create_int!(self, i8)),
            BasicInput::U16 => MoveValue::U16(create_int!(self, u16)),
            BasicInput::I16 => MoveValue::I16(create_int!(self, i16)),
            BasicInput::U32 => MoveValue::U32(create_int!(self, u32)),
            BasicInput::I32 => MoveValue::I32(create_int!(self, i32)),
            BasicInput::U64 => MoveValue::U64(create_int!(self, u64)),
            BasicInput::I64 => MoveValue::I64(create_int!(self, i64)),
            BasicInput::U128 => MoveValue::U128(create_int!(self, u128)),
            BasicInput::I128 => MoveValue::I128(create_int!(self, i128)),
            BasicInput::U256 => MoveValue::U256(create_int!(
                self,
                U256::ZERO,
                U256::MAX,
                U256::from_le_bytes(self.rng.r#gen())
            )),
            BasicInput::I256 => MoveValue::I256(create_int!(
                self,
                I256::MIN,
                I256::MAX,
                I256::from_le_bytes(self.rng.r#gen())
            )),
            BasicInput::String => self.random_string(),
            BasicInput::Address => MoveValue::Address(self.random_address()),
            BasicInput::Signer => MoveValue::Signer(self.random_signer()),
            BasicInput::ObjectKnown { ident, .. } => {
                MoveValue::Address(self.random_object_address_for(ident))
            },
            BasicInput::ObjectParam { .. } => MoveValue::Address(self.random_object_address_any()),
            BasicInput::Vector(element) => {
                let size = self.rng.gen_range(0, GEN_VEC_SIZE_MAX);
                MoveValue::Vector((0..size).map(|_| self.random_value(element)).collect())
            },
        }
    }

    /// Randomly mutate a Move value based on a basic input type
    pub fn mutate_value(&mut self, ty: &BasicInput, val: &MoveValue) -> MoveValue {
        match ty {
            BasicInput::Bool => match val {
                MoveValue::Bool(v) => MoveValue::Bool(!v),
                _ => self.random_value(ty),
            },
            BasicInput::U8 => match val {
                MoveValue::U8(v) => MoveValue::U8(mutate_int_with_edges!(self, *v, u8)),
                _ => self.random_value(ty),
            },
            BasicInput::I8 => match val {
                MoveValue::I8(v) => MoveValue::I8(mutate_int_with_edges!(self, *v, i8)),
                _ => self.random_value(ty),
            },
            BasicInput::U16 => match val {
                MoveValue::U16(v) => MoveValue::U16(mutate_int_with_edges!(self, *v, u16)),
                _ => self.random_value(ty),
            },
            BasicInput::I16 => match val {
                MoveValue::I16(v) => MoveValue::I16(mutate_int_with_edges!(self, *v, i16)),
                _ => self.random_value(ty),
            },
            BasicInput::U32 => match val {
                MoveValue::U32(v) => MoveValue::U32(mutate_int_with_edges!(self, *v, u32)),
                _ => self.random_value(ty),
            },
            BasicInput::I32 => match val {
                MoveValue::I32(v) => MoveValue::I32(mutate_int_with_edges!(self, *v, i32)),
                _ => self.random_value(ty),
            },
            BasicInput::U64 => match val {
                MoveValue::U64(v) => MoveValue::U64(mutate_int_with_edges!(self, *v, u64)),
                _ => self.random_value(ty),
            },
            BasicInput::I64 => match val {
                MoveValue::I64(v) => MoveValue::I64(mutate_int_with_edges!(self, *v, i64)),
                _ => self.random_value(ty),
            },
            BasicInput::U128 => match val {
                MoveValue::U128(v) => MoveValue::U128(mutate_int_with_edges!(self, *v, u128)),
                _ => self.random_value(ty),
            },
            BasicInput::I128 => match val {
                MoveValue::I128(v) => MoveValue::I128(mutate_int_with_edges!(self, *v, i128)),
                _ => self.random_value(ty),
            },
            BasicInput::U256 => match val {
                MoveValue::U256(v) => MoveValue::U256(mutate_int256!(self, U256, *v)),
                _ => self.random_value(ty),
            },
            BasicInput::I256 => match val {
                MoveValue::I256(v) => MoveValue::I256(mutate_int256!(self, I256, *v)),
                _ => self.random_value(ty),
            },
            BasicInput::String => match val {
                MoveValue::Vector(elements)
                    if elements.iter().all(|e| matches!(e, MoveValue::U8(_))) =>
                {
                    self.mutate_string(elements)
                },
                _ => self.random_string(),
            },
            BasicInput::Address => match val {
                MoveValue::Address(addr) => MoveValue::Address(self.mutate_address(*addr)),
                _ => MoveValue::Address(self.random_address()),
            },
            BasicInput::Signer => match val {
                MoveValue::Signer(addr) => MoveValue::Signer(self.mutate_signer(*addr)),
                MoveValue::Address(addr) => MoveValue::Signer(self.mutate_signer(*addr)),
                _ => MoveValue::Signer(self.random_signer()),
            },
            BasicInput::ObjectKnown { ident, .. } => {
                let current = match val {
                    MoveValue::Address(addr) | MoveValue::Signer(addr) => Some(*addr),
                    _ => None,
                };
                MoveValue::Address(self.mutate_object_address(ident, current))
            },
            BasicInput::ObjectParam { .. } => {
                let current = match val {
                    MoveValue::Address(addr) | MoveValue::Signer(addr) => Some(*addr),
                    _ => None,
                };
                MoveValue::Address(self.mutate_any_object_address(current))
            },
            BasicInput::Vector(element) => match val {
                MoveValue::Vector(elements) => self.mutate_vector(element, elements),
                _ => self.random_value(ty),
            },
        }
    }

    /// Decide whether to mutate type arguments (~30% probability)
    pub fn should_mutate_type_args(&mut self) -> bool {
        self.rng.gen_range(0, 100) < MUT_TYPE_ARG_PROB
    }

    /// Randomly generate type arguments satisfying the given ability constraints
    pub fn random_type_args(&mut self, generics: &[AbilitySet]) -> Vec<VmTypeTag> {
        generics
            .iter()
            .map(|constraint| {
                let candidates = self.type_pool.candidates_for(*constraint);
                let index = self.rng.gen_range(0, candidates.len());
                candidates[index].clone()
            })
            .collect()
    }

    /// Mutate type arguments by randomly replacing one type parameter with a different candidate
    pub fn mutate_type_args(
        &mut self,
        generics: &[AbilitySet],
        current: &[VmTypeTag],
    ) -> Vec<VmTypeTag> {
        assert_eq!(generics.len(), current.len());
        if generics.is_empty() {
            return vec![];
        }

        let mut result = current.to_vec();
        let mut positions: Vec<_> = (0..generics.len()).collect();
        positions.shuffle(&mut self.rng);

        let mut positions_to_mutate = 1usize;
        while positions_to_mutate < generics.len() && self.random_percent() < 35 {
            positions_to_mutate += 1;
        }

        let mut changed = false;
        for pos in positions.into_iter().take(positions_to_mutate) {
            let candidates = self.type_pool.candidates_for(generics[pos]);
            let alternatives: Vec<_> = candidates
                .into_iter()
                .filter(|candidate| **candidate != current[pos])
                .collect();
            if alternatives.is_empty() {
                continue;
            }
            let replacement = alternatives[self.rng.gen_range(0, alternatives.len())];
            result[pos] = (*replacement).clone();
            changed = true;
        }

        if !changed {
            let pos = self.rng.gen_range(0, generics.len());
            let candidates = self.type_pool.candidates_for(generics[pos]);
            if let Some(candidate) = candidates.first() {
                result[pos] = (**candidate).clone();
            }
        }
        result
    }

    /// Randomly generate a Move string value
    fn random_string(&mut self) -> MoveValue {
        let x = self.rng.gen_range(0, GEN_STR_PROB_TOTAL);
        if x < GEN_STR_PROB_DICT {
            let idx = self.rng.gen_range(0, self.dict_string.len());
            str_to_move_bytes(&self.dict_string[idx])
        } else if x < GEN_STR_PROB_DICT + GEN_STR_PROB_EMPTY {
            MoveValue::Vector(vec![])
        } else {
            let len = self.rng.gen_range(1, GEN_STR_MAX_LEN + 1);
            MoveValue::Vector(
                (0..len)
                    .map(|_| MoveValue::U8(self.random_ascii_byte()))
                    .collect(),
            )
        }
    }

    /// Mutate a Move string (represented as vector<u8> elements)
    fn mutate_string(&mut self, elements: &[MoveValue]) -> MoveValue {
        let mut bytes: Vec<u8> = elements
            .iter()
            .filter_map(|value| match value {
                MoveValue::U8(byte) => Some(*byte),
                _ => None,
            })
            .collect();
        let x = self.rng.gen_range(0, MUT_STR_PROB_TOTAL);

        if x < MUT_STR_PROB_ADD && bytes.len() < GEN_STR_MAX_LEN {
            let pos = self.rng.gen_range(0, bytes.len() + 1);
            bytes.insert(pos, self.random_ascii_byte());
            move_bytes(bytes)
        } else if x < MUT_STR_PROB_ADD + MUT_STR_PROB_DEL && !bytes.is_empty() {
            let pos = self.rng.gen_range(0, bytes.len());
            bytes.remove(pos);
            move_bytes(bytes)
        } else if x < MUT_STR_PROB_ADD + MUT_STR_PROB_DEL + MUT_STR_PROB_CHANGE && !bytes.is_empty()
        {
            let pos = self.rng.gen_range(0, bytes.len());
            bytes[pos] = self.random_ascii_byte();
            move_bytes(bytes)
        } else if x < MUT_STR_PROB_ADD + MUT_STR_PROB_DEL + MUT_STR_PROB_CHANGE + MUT_STR_PROB_DICT
        {
            let idx = self.rng.gen_range(0, self.dict_string.len());
            str_to_move_bytes(&self.dict_string[idx])
        } else {
            let fragment = self.random_string_fragment();
            let pos = self.rng.gen_range(0, bytes.len() + 1);
            bytes.splice(pos..pos, fragment);
            bytes.truncate(GEN_STR_MAX_LEN);
            move_bytes(bytes)
        }
    }

    fn mutate_vector(&mut self, elem_ty: &BasicInput, elements: &[MoveValue]) -> MoveValue {
        if elements.is_empty() {
            return MoveValue::Vector(vec![self.random_value(elem_ty)]);
        }

        let mut new_elements = elements.to_vec();
        let x = self.rng.gen_range(0, MUT_VEC_PROB_TOTAL);
        if x < MUT_VEC_PROB_ADD_ELEMENT && new_elements.len() < GEN_VEC_SIZE_MAX as usize {
            let pos = self.rng.gen_range(0, new_elements.len() + 1);
            new_elements.insert(pos, self.random_value(elem_ty));
        } else if x < MUT_VEC_PROB_ADD_ELEMENT + MUT_VEC_PROB_DEL_ELEMENT {
            let pos = self.rng.gen_range(0, new_elements.len());
            new_elements.remove(pos);
        } else if x < MUT_VEC_PROB_ADD_ELEMENT + MUT_VEC_PROB_DEL_ELEMENT + MUT_VEC_PROB_MUT_ELEMENT
        {
            let pos = self.rng.gen_range(0, new_elements.len());
            let current = &new_elements[pos];
            new_elements[pos] = self.mutate_value(elem_ty, current);
        } else if x < MUT_VEC_PROB_ADD_ELEMENT
            + MUT_VEC_PROB_DEL_ELEMENT
            + MUT_VEC_PROB_MUT_ELEMENT
            + MUT_VEC_PROB_DUP_ELEMENT
            && new_elements.len() < GEN_VEC_SIZE_MAX as usize
        {
            let pos = self.rng.gen_range(0, new_elements.len());
            let value = new_elements[pos].clone();
            new_elements.insert(pos, value);
        } else if x < MUT_VEC_PROB_ADD_ELEMENT
            + MUT_VEC_PROB_DEL_ELEMENT
            + MUT_VEC_PROB_MUT_ELEMENT
            + MUT_VEC_PROB_DUP_ELEMENT
            + MUT_VEC_PROB_SWAP_ELEMENTS
            && new_elements.len() > 1
        {
            let first = self.rng.gen_range(0, new_elements.len());
            let mut second = self.rng.gen_range(0, new_elements.len());
            while second == first {
                second = self.rng.gen_range(0, new_elements.len());
            }
            new_elements.swap(first, second);
        } else if new_elements.len() > 1 {
            if self.random_percent() < 50 {
                new_elements.reverse();
            } else {
                let shift = self.rng.gen_range(1, new_elements.len());
                new_elements.rotate_left(shift);
            }
        } else {
            new_elements[0] = self.mutate_value(elem_ty, &new_elements[0]);
        }

        MoveValue::Vector(new_elements)
    }

    /// Generate a random printable ASCII byte
    fn random_ascii_byte(&mut self) -> u8 {
        self.rng.gen_range(0x20u8, 0x7Fu8)
    }

    fn random_string_fragment(&mut self) -> Vec<u8> {
        if !self.dict_string.is_empty() && self.random_percent() < 50 {
            self.dict_string[self.rng.gen_range(0, self.dict_string.len())]
                .bytes()
                .take(self.rng.gen_range(1, GEN_STR_MAX_LEN + 1))
                .collect()
        } else {
            let len = self.rng.gen_range(1, GEN_STR_MAX_LEN.min(8) + 1);
            (0..len).map(|_| self.random_ascii_byte()).collect()
        }
    }

    fn mutate_address(&mut self, current: AccountAddress) -> AccountAddress {
        let choice = self.random_percent();
        if choice < 20 {
            current
        } else if choice < 75 && !self.object_addresses.is_empty() {
            self.random_object_address_any()
        } else {
            self.random_address()
        }
    }

    fn mutate_signer(&mut self, current: AccountAddress) -> AccountAddress {
        if self.random_percent() < 20 {
            current
        } else {
            self.random_signer()
        }
    }

    fn mutate_object_address(
        &mut self,
        ident: &DatatypeIdent,
        current: Option<AccountAddress>,
    ) -> AccountAddress {
        if let Some(addr) = current {
            if self.random_percent() < 20 {
                return addr;
            }
        }
        if self.random_percent() < 80 {
            self.random_object_address_for(ident)
        } else {
            self.random_address()
        }
    }

    fn mutate_any_object_address(&mut self, current: Option<AccountAddress>) -> AccountAddress {
        if let Some(addr) = current {
            if self.random_percent() < 20 {
                return addr;
            }
        }
        if self.random_percent() < 80 {
            self.random_object_address_any()
        } else {
            self.random_address()
        }
    }

    fn random_account_address(&mut self) -> AccountAddress {
        loop {
            let x = self.rng.gen_range(0, GEN_ADDR_PROB_TOTAL);
            let kind = if x < GEN_ADDR_PROB_NAME_PRIMARY {
                AddressKind::Named(NamedAddressKind::Primary)
            } else if x < GEN_ADDR_PROB_NAME_PRIMARY + GEN_ADDR_PROB_NAME_DEPENDENCY {
                AddressKind::Named(NamedAddressKind::Dependency)
            } else if x < GEN_ADDR_PROB_NAME_PRIMARY
                + GEN_ADDR_PROB_NAME_DEPENDENCY
                + GEN_ADDR_PROB_NAME_FRAMEWORK
            {
                AddressKind::Named(NamedAddressKind::Framework)
            } else {
                AddressKind::User
            };

            let addrs = match self.dict_address.get(&kind) {
                None => continue,
                Some(v) => v,
            };
            if addrs.is_empty() {
                continue;
            }

            let index = self.rng.gen_range(0, addrs.len());
            return *addrs.iter().nth(index).unwrap();
        }
    }

    /// Get a random address.
    ///
    /// Plain `address` arguments often reference object addresses rather than
    /// signer-capable accounts, so reuse discovered objects when available.
    fn random_address(&mut self) -> AccountAddress {
        if !self.object_addresses.is_empty() && self.random_percent() < GEN_ADDR_PROB_OBJECT {
            return self.random_object_address_any();
        }
        self.random_account_address()
    }

    /// Get a random address out of which we can create a signer
    pub fn random_signer(&mut self) -> AccountAddress {
        self.random_account_address()
    }

    /// Generate a random percentage value in [0, 100)
    pub fn random_percent(&mut self) -> u8 {
        self.rng.gen_range(0u8, 100)
    }

    /// Get a mutable reference to the internal RNG
    pub fn rng_mut(&mut self) -> &mut StdRng {
        &mut self.rng
    }

    /// Update the object dictionary from write set resource writes.
    ///
    /// Pass 1: identify object addresses (those with ObjectGroup resource group).
    /// Pass 2: record resource-type-to-address mappings for known objects.
    pub fn update_object_dict(&mut self, writes: &[ResourceWrite]) {
        // Pass 1: find new object addresses via ObjectGroup resource group writes
        for ResourceWrite {
            address,
            struct_tag,
            is_resource_group,
        } in writes
        {
            if *is_resource_group
                && struct_tag.address == AccountAddress::ONE
                && struct_tag.module.as_str() == "object"
                && struct_tag.name.as_str() == "ObjectGroup"
            {
                self.object_addresses.insert(*address);
            }
        }

        // Pass 2: for resources written at known object addresses, record the mapping
        for ResourceWrite {
            address,
            struct_tag,
            is_resource_group,
        } in writes
        {
            if !is_resource_group && self.object_addresses.contains(address) {
                let ident = DatatypeIdent::from_struct_tuple(
                    struct_tag.address,
                    struct_tag.module.clone(),
                    struct_tag.name.clone(),
                );
                self.dict_object.entry(ident).or_default().insert(*address);
            }
        }
    }

    /// Export the discovered object-address state for persistence.
    pub fn snapshot_object_state(&self) -> PersistedObjectState {
        PersistedObjectState {
            object_addresses: self.object_addresses.clone(),
            dict_object: self
                .dict_object
                .iter()
                .map(|(ident, addrs)| PersistedObjectBucket {
                    ident: PersistedDatatypeIdent::from_ident(ident),
                    addresses: addrs.clone(),
                })
                .collect(),
        }
    }

    /// Restore the discovered object-address state from persistence.
    pub fn restore_object_state(&mut self, state: &PersistedObjectState) -> Result<()> {
        self.object_addresses = state.object_addresses.clone();
        self.dict_object.clear();
        for bucket in &state.dict_object {
            let ident = bucket.ident.clone().into_ident()?;
            self.dict_object
                .entry(ident)
                .or_default()
                .extend(bucket.addresses.iter().copied());
        }
        Ok(())
    }

    /// Get a random object address for a known object type
    fn random_object_address_for(&mut self, ident: &DatatypeIdent) -> AccountAddress {
        if let Some(addrs) = self.dict_object.get(ident) {
            if !addrs.is_empty() {
                let index = self.rng.gen_range(0, addrs.len());
                return *addrs.iter().nth(index).unwrap();
            }
        }
        self.random_object_address_any()
    }

    /// Get a random object address from any known object
    fn random_object_address_any(&mut self) -> AccountAddress {
        if !self.object_addresses.is_empty() {
            let index = self.rng.gen_range(0, self.object_addresses.len());
            return *self.object_addresses.iter().nth(index).unwrap();
        }
        self.random_address()
    }
}

/// Utility: convert a string to a Move byte vector
#[inline]
fn str_to_move_bytes(s: &str) -> MoveValue {
    move_bytes(s.bytes().collect())
}

#[inline]
fn move_bytes(bytes: Vec<u8>) -> MoveValue {
    MoveValue::Vector(bytes.into_iter().map(MoveValue::U8).collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::prep::canvas::BasicInput;

    fn test_mutator_with_address_buckets(seed: u64) -> Mutator {
        let mut dict_address: BTreeMap<AddressKind, BTreeSet<AccountAddress>> = BTreeMap::new();
        dict_address.insert(
            AddressKind::Named(NamedAddressKind::Primary),
            BTreeSet::from([AccountAddress::from_hex_literal("0x11").unwrap()]),
        );
        dict_address.insert(
            AddressKind::Named(NamedAddressKind::Dependency),
            BTreeSet::from([AccountAddress::from_hex_literal("0x22").unwrap()]),
        );
        dict_address.insert(
            AddressKind::Named(NamedAddressKind::Framework),
            BTreeSet::from([AccountAddress::from_hex_literal("0x33").unwrap()]),
        );
        dict_address.insert(
            AddressKind::User,
            BTreeSet::from([AccountAddress::from_hex_literal("0x44").unwrap()]),
        );
        Mutator::new(seed, dict_address, TypePool::new(), vec![])
    }

    #[test]
    fn test_random_int_generation_reaches_non_extreme_values() {
        let mut mutator = test_mutator_with_address_buckets(7);
        let mut saw_min = false;
        let mut saw_max = false;
        let mut saw_mid = false;

        for _ in 0..2000 {
            let v = match mutator.random_value(&BasicInput::U64) {
                MoveValue::U64(v) => v,
                _ => unreachable!("expected u64"),
            };
            saw_min |= v == u64::MIN;
            saw_max |= v == u64::MAX;
            saw_mid |= v != u64::MIN && v != u64::MAX;
            if saw_min && saw_max && saw_mid {
                break;
            }
        }

        assert!(saw_min, "u64::MIN branch was not sampled");
        assert!(saw_max, "u64::MAX branch was not sampled");
        assert!(saw_mid, "random mid-value branch was not sampled");
    }

    #[test]
    fn test_random_address_samples_all_kinds() {
        let mut mutator = test_mutator_with_address_buckets(42);
        let primary = AccountAddress::from_hex_literal("0x11").unwrap();
        let dependency = AccountAddress::from_hex_literal("0x22").unwrap();
        let framework = AccountAddress::from_hex_literal("0x33").unwrap();
        let user = AccountAddress::from_hex_literal("0x44").unwrap();

        let mut seen_primary = false;
        let mut seen_dependency = false;
        let mut seen_framework = false;
        let mut seen_user = false;

        for _ in 0..2000 {
            let addr = mutator.random_signer();
            seen_primary |= addr == primary;
            seen_dependency |= addr == dependency;
            seen_framework |= addr == framework;
            seen_user |= addr == user;
            if seen_primary && seen_dependency && seen_framework && seen_user {
                break;
            }
        }

        assert!(seen_primary, "primary address bucket was never sampled");
        assert!(
            seen_dependency,
            "dependency address bucket was never sampled"
        );
        assert!(seen_framework, "framework address bucket was never sampled");
        assert!(seen_user, "user address bucket was never sampled");
    }

    #[test]
    fn test_mutate_signer_preserves_signer_shape() {
        let mut mutator = test_mutator_with_address_buckets(1);
        let mutated = mutator.mutate_value(
            &BasicInput::Signer,
            &MoveValue::Signer(AccountAddress::from_hex_literal("0x44").unwrap()),
        );
        assert!(matches!(mutated, MoveValue::Signer(_)));
    }

    #[test]
    fn test_mutate_value_regenerates_on_type_mismatch() {
        let mut mutator = test_mutator_with_address_buckets(2);
        let mutated = mutator.mutate_value(
            &BasicInput::U64,
            &MoveValue::Struct(move_core_types::value::MoveStruct::Runtime(vec![])),
        );
        assert!(matches!(mutated, MoveValue::U64(_)));
    }

    #[test]
    fn test_mutate_string_with_splice_keeps_vector_u8_shape() {
        let mut mutator = test_mutator_with_address_buckets(9);
        let mutated = mutator.mutate_value(
            &BasicInput::String,
            &MoveValue::Vector(vec![MoveValue::U8(b'a'), MoveValue::U8(b'b')]),
        );
        match mutated {
            MoveValue::Vector(values) => {
                assert!(values.iter().all(|value| matches!(value, MoveValue::U8(_))));
            },
            other => panic!("expected vector<u8> string, got {other:?}"),
        }
    }

    #[test]
    fn test_mutate_vector_preserves_vector_shape() {
        let mut mutator = test_mutator_with_address_buckets(17);
        let mutated = mutator.mutate_value(
            &BasicInput::Vector(Box::new(BasicInput::U8)),
            &MoveValue::Vector(vec![MoveValue::U8(1), MoveValue::U8(2), MoveValue::U8(3)]),
        );
        match mutated {
            MoveValue::Vector(values) => {
                assert!(values.iter().all(|value| matches!(value, MoveValue::U8(_))));
            },
            other => panic!("expected vector mutation, got {other:?}"),
        }
    }

    #[test]
    fn test_object_state_roundtrip() -> Result<()> {
        let mut mutator = test_mutator_with_address_buckets(99);
        let object_addr = AccountAddress::from_hex_literal("0xabc").unwrap();
        mutator.object_addresses.insert(object_addr);
        mutator.dict_object.insert(
            DatatypeIdent::from_struct_tuple(
                AccountAddress::ONE,
                move_core_types::identifier::Identifier::new("m").unwrap(),
                move_core_types::identifier::Identifier::new("Vault").unwrap(),
            ),
            BTreeSet::from([object_addr]),
        );

        let snapshot = mutator.snapshot_object_state();
        let mut restored = test_mutator_with_address_buckets(100);
        restored.restore_object_state(&snapshot)?;
        assert!(restored.object_addresses.contains(&object_addr));
        assert_eq!(restored.dict_object.len(), 1);
        Ok(())
    }

    #[test]
    fn test_plain_address_generation_reuses_known_objects() {
        let mut mutator = test_mutator_with_address_buckets(123);
        let object_addr = AccountAddress::from_hex_literal("0xabc").unwrap();
        mutator.object_addresses.insert(object_addr);

        let mut saw_object = false;
        for _ in 0..512 {
            if let MoveValue::Address(addr) = mutator.random_value(&BasicInput::Address) {
                if addr == object_addr {
                    saw_object = true;
                    break;
                }
            }
        }

        assert!(
            saw_object,
            "plain address generation never reused a known object"
        );
    }
}
