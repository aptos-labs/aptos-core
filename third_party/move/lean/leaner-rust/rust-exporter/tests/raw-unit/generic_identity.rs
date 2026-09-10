// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

pub fn identity<T>(value: T) -> T {
    value
}

pub fn choose<T>(value: &T) -> &T {
    value
}

pub fn round_trip<T>(value: T) -> T {
    identity(value)
}
