// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

pub fn nested(rows: u32, columns: u32) -> u32 {
    let mut total = 0;
    let mut row = 0;
    while row < rows {
        let mut column = 0;
        while column < columns {
            total += 1;
            column += 1;
        }
        row += 1;
    }
    total
}

pub fn skip_rows(rows: u32, columns: u32) -> u32 {
    let mut count = 0;
    let mut row = 0;
    'outer: while row < rows {
        row += 1;
        let mut column = 0;
        while column < columns {
            column += 1;
            if column == row {
                continue 'outer;
            }
            count += 1;
        }
        count += 2;
    }
    count
}

pub fn classify(n: u32, limit: u32) -> u32 {
    let mut index = 0;
    let kind = loop {
        if index >= n {
            break 0;
        }
        if index > limit {
            break 1;
        }
        index += 1;
    };
    let mut total = kind;
    while total < limit {
        total += 1;
    }
    total
}

pub fn facts(n: u32) -> u32 {
    let mut i = 0;
    while i < n {
        i += 1;
    }
    i
}
