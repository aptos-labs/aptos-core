// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

#![allow(invalid_from_utf8)]

pub fn valid_empty() -> bool {
    let bytes = [];
    std::str::from_utf8(&bytes).is_ok()
}

pub fn valid_ascii() -> bool {
    let bytes = [0x41, 0x0, 0x7f];
    std::str::from_utf8(&bytes).is_ok()
}

pub fn valid_two_byte() -> bool {
    let bytes = [0xc3, 0xa9];
    std::str::from_utf8(&bytes).is_ok()
}

pub fn valid_three_byte() -> bool {
    let bytes = [0xe4, 0xb8, 0xad];
    std::str::from_utf8(&bytes).is_ok()
}

pub fn valid_four_byte() -> bool {
    let bytes = [0xf0, 0x9f, 0xa6, 0x80];
    std::str::from_utf8(&bytes).is_ok()
}

pub fn invalid_ff() -> bool {
    let bytes = [0xff];
    std::str::from_utf8(&bytes).is_err()
}

pub fn invalid_continuation() -> bool {
    let bytes = [0x80];
    std::str::from_utf8(&bytes).is_err()
}

pub fn invalid_overlong() -> bool {
    let bytes = [0xc0, 0xaf];
    std::str::from_utf8(&bytes).is_err()
}

pub fn invalid_truncated() -> bool {
    let bytes = [0xe2, 0x82];
    std::str::from_utf8(&bytes).is_err()
}

pub fn invalid_surrogate() -> bool {
    let bytes = [0xed, 0xa0, 0x80];
    std::str::from_utf8(&bytes).is_err()
}

pub fn invalid_beyond_unicode() -> bool {
    let bytes = [0xf4, 0x90, 0x80, 0x80];
    std::str::from_utf8(&bytes).is_err()
}
