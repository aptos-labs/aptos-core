// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

module 0x42::enum_reference_patterns {
    // The same field name has distinct generic types in the two variants.
    // An unqualified projection cannot replace this typed pattern.
    enum Either<T, E> has copy, drop {
        Left { value: T },
        Right { value: E },
    }

    fun unwrap_left<T, E>(value: Either<T, E>): T {
        match (value) {
            Either::Left { value } => value,
            _ => abort 7,
        }
    }

    fun unwrap_left_u64(value: Either<u64, u64>): u64 {
        match (value) {
            Either::Left { value } => value,
            _ => abort 7,
        }
    }

    enum Counter has copy, drop {
        One { value: u64 },
        Two { value: u64 },
    }

    fun counter(value: Counter): u64 {
        match (value) {
            Counter::One { value } => value,
            Counter::Two { value } => value,
        }
    }

    enum Slot<T> has copy, drop {
        Empty,
        Filled { value: T },
    }

    fun borrow_or<T>(slot: &Slot<T>, fallback: &T): &T {
        match (slot) {
            Slot::Empty => fallback,
            Slot::Filled { value } => value,
        }
    }

    fun value_or<T: copy + drop>(slot: &Slot<T>, fallback: T): T {
        match (slot) {
            Slot::Empty => fallback,
            Slot::Filled { value } => *value,
        }
    }

    fun borrow_mut<T>(slot: &mut Slot<T>): &mut T {
        match (slot) {
            Slot::Empty => abort 7,
            Slot::Filled { value } => value,
        }
    }

    fun replace(slot: &mut Slot<u64>, replacement: u64): u64 {
        match (slot) {
            Slot::Empty => abort 7,
            Slot::Filled { value } => {
                let previous = *value;
                *value = replacement;
                previous
            },
        }
    }
}
