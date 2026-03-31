// Parts of the file are Copyright (c) The Diem Core Contributors
// Parts of the file are Copyright (c) The Move Contributors
// Parts of the file are Copyright (c) Aptos Foundation
// All Aptos Foundation code and content is licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{diag, diagnostics::Diagnostic, parser::syntax::make_loc};
use move_ir_types::location::*;

pub fn decode(loc: Loc, s: &str) -> Result<Vec<u8>, Box<Diagnostic>> {
    match hex::decode(s) {
        Ok(vec) => Ok(vec),
        Err(hex::FromHexError::InvalidHexCharacter { c, index }) => {
            let filename = loc.file_hash();
            let start_offset = loc.start() as usize;
            let offset = start_offset + 2 + index;
            let loc = make_loc(filename, offset, offset);
            Err(Box::new(diag!(
                Syntax::InvalidHexString,
                (loc, format!("Invalid hexadecimal character: '{}'", c)),
            )))
        },
        Err(hex::FromHexError::OddLength) => Err(Box::new(diag!(
            Syntax::InvalidHexString,
            (
                loc,
                "Odd number of characters in hex string. Expected 2 hexadecimal digits for each \
                 byte"
                    .to_string(),
            )
        ))),
        Err(_) => unreachable!("unexpected error parsing hex byte string value"),
    }
}
