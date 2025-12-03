// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

pub fn strip_trailing_zeros_and_decimal_point(mut s: &str) -> &str {
    loop {
        match s {
            "0" | ".0" => return s,
            _ => match s.strip_suffix('0') {
                Some(stripped) => s = stripped,
                None => break,
            },
        }
    }
    match s.strip_suffix('.') {
        Some(stripped) => stripped,
        None => s,
    }
}
