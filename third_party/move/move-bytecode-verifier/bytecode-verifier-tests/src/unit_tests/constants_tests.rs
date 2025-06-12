// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

#![allow(unexpected_cfgs)]

use move_binary_format::file_format::{empty_module, CompiledModule, Constant, SignatureToken};
use move_bytecode_verifier::constants;
use move_core_types::vm_status::StatusCode;
use proptest::prelude::*;

proptest! {
    #[test]
    fn valid_generated(module in CompiledModule::valid_strategy(20)) {
        prop_assert!(constants::verify_module(&module).is_ok());
    }
}

#[test]
#[cfg(not(feature = "address32"))]
fn valid_primitives() {
    let mut module = empty_module();
    module.constant_pool = vec![
        Constant {
            type_: SignatureToken::Bool,
            data: vec![0],
        },
        Constant {
            type_: SignatureToken::U8,
            data: vec![0],
        },
        Constant {
            type_: SignatureToken::U16,
            data: vec![0, 0],
        },
        Constant {
            type_: SignatureToken::U32,
            data: vec![0, 0, 0, 0],
        },
        Constant {
            type_: SignatureToken::U64,
            data: vec![0, 0, 0, 0, 0, 0, 0, 0],
        },
        Constant {
            type_: SignatureToken::U128,
            data: vec![0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
        },
        Constant {
            type_: SignatureToken::U256,
            data: vec![
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                0, 0, 0, 0,
            ],
        },
        Constant {
            type_: SignatureToken::Address,
            data: vec![
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                0, 0, 0, 0,
            ],
        },
    ];
    assert!(constants::verify_module(&module).is_ok());
}

#[test]
#[cfg(not(feature = "address32"))]
fn invalid_primitives() {
    malformed(SignatureToken::U8, vec![0, 0]);
    malformed(SignatureToken::U16, vec![0, 0, 0, 0]);
    malformed(SignatureToken::U32, vec![0, 0, 0]);
    malformed(SignatureToken::U64, vec![0]);
    malformed(SignatureToken::U128, vec![0]);
    malformed(SignatureToken::U256, vec![0, 0]);
    let data = vec![
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 0,
    ];
    malformed(SignatureToken::Address, data);
}

#[test]
#[cfg(not(feature = "address32"))]
fn valid_vectors() {
    let double_vec = |item: Vec<u8>| -> Vec<u8> {
        let mut items = vec![2];
        items.extend(item.clone());
        items.extend(item);
        items
    };
    let large_vec = |item: Vec<u8>| -> Vec<u8> {
        let mut items = vec![0xFF, 0xFF, 3];
        (0..0xFFFF).for_each(|_| items.extend(item.clone()));
        items
    };
    let mut module = empty_module();
    module.constant_pool = vec![
        // empty
        Constant {
            type_: tvec(SignatureToken::Bool),
            data: vec![0],
        },
        Constant {
            type_: tvec(tvec(SignatureToken::Bool)),
            data: vec![0],
        },
        Constant {
            type_: tvec(tvec(tvec(tvec(SignatureToken::Bool)))),
            data: vec![0],
        },
        Constant {
            type_: tvec(tvec(tvec(tvec(SignatureToken::Bool)))),
            data: double_vec(vec![0]),
        },
        // small
        Constant {
            type_: tvec(SignatureToken::Bool),
            data: vec![9, 1, 1, 1, 1, 1, 1, 1, 1, 1],
        },
        Constant {
            type_: tvec(SignatureToken::U8),
            data: vec![9, 1, 1, 1, 1, 1, 1, 1, 1, 1],
        },
        // large
        Constant {
            type_: tvec(SignatureToken::Bool),
            data: large_vec(vec![0]),
        },
        Constant {
            type_: tvec(SignatureToken::U8),
            data: large_vec(vec![0]),
        },
        Constant {
            type_: tvec(SignatureToken::U16),
            data: large_vec(vec![0, 0]),
        },
        Constant {
            type_: tvec(SignatureToken::U32),
            data: large_vec(vec![0, 0, 0, 0]),
        },
        Constant {
            type_: tvec(SignatureToken::U64),
            data: large_vec(vec![0, 0, 0, 0, 0, 0, 0, 0]),
        },
        Constant {
            type_: tvec(SignatureToken::U128),
            data: large_vec(vec![0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]),
        },
        Constant {
            type_: tvec(SignatureToken::U256),
            data: large_vec(vec![
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                0, 0, 0, 0,
            ]),
        },
        Constant {
            type_: tvec(SignatureToken::Address),
            data: large_vec(vec![
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                0, 0, 0, 0,
            ]),
        },
        // double large
        Constant {
            type_: tvec(tvec(SignatureToken::Bool)),
            data: double_vec(large_vec(vec![0])),
        },
        Constant {
            type_: tvec(tvec(SignatureToken::U8)),
            data: double_vec(large_vec(vec![0])),
        },
        Constant {
            type_: tvec(tvec(SignatureToken::U16)),
            data: double_vec(large_vec(vec![0, 0])),
        },
        Constant {
            type_: tvec(tvec(SignatureToken::U32)),
            data: double_vec(large_vec(vec![0, 0, 0, 0])),
        },
        Constant {
            type_: tvec(tvec(SignatureToken::U64)),
            data: double_vec(large_vec(vec![0, 0, 0, 0, 0, 0, 0, 0])),
        },
        Constant {
            type_: tvec(tvec(SignatureToken::U128)),
            data: double_vec(large_vec(vec![
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            ])),
        },
        Constant {
            type_: tvec(tvec(SignatureToken::U256)),
            data: double_vec(large_vec(vec![
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                0, 0, 0, 0,
            ])),
        },
        Constant {
            type_: tvec(tvec(SignatureToken::Address)),
            data: double_vec(large_vec(vec![
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                0, 0, 0, 0,
            ])),
        },
    ];
    assert!(constants::verify_module(&module).is_ok());
}

#[test]
fn invalid_vectors() {
    let double_vec = |item: Vec<u8>| -> Vec<u8> {
        let mut items = vec![2];
        items.extend(item.clone());
        items.extend(item);
        items
    };
    let too_large_vec = |item: Vec<u8>| -> Vec<u8> {
        let mut items = vec![0xFF, 0xFF, 3];
        (0..(0xFFFF + 1)).for_each(|_| items.extend(item.clone()));
        items
    };
    // wrong inner
    malformed(tvec(SignatureToken::U16), vec![1, 0]);
    malformed(tvec(SignatureToken::U32), vec![1, 0]);
    malformed(tvec(SignatureToken::U64), vec![1, 0]);
    malformed(tvec(SignatureToken::Address), vec![
        1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 0, 0, 0,
    ]);
    // wrong lens
    malformed(tvec(SignatureToken::U8), vec![0, 0]);
    malformed(tvec(SignatureToken::U8), vec![0, 1]);
    malformed(tvec(SignatureToken::U8), vec![2, 1, 1, 1]);
    malformed(tvec(tvec(SignatureToken::U8)), double_vec(vec![0, 0]));
    // too large
    malformed(tvec(SignatureToken::U8), too_large_vec(vec![0]));
}

#[test]
fn invalid_types() {
    invalid_type(SignatureToken::TypeParameter(0), vec![0]);
    invalid_type(SignatureToken::TypeParameter(0xFA), vec![0]);
    invalid_type(tvec(SignatureToken::TypeParameter(0)), vec![0]);
    invalid_type(tvec(SignatureToken::TypeParameter(0xAF)), vec![0]);

    invalid_type(SignatureToken::Signer, vec![0]);
    invalid_type(tvec(SignatureToken::Signer), vec![0]);

    // TODO cannot check structs are banned currently. This can be handled by IR and source lang
    // tests
    // invalid_type(SignatureToken::Struct(StructHandleIndex(0)), vec![0]);
}

fn tvec(s: SignatureToken) -> SignatureToken {
    SignatureToken::Vector(Box::new(s))
}

#[allow(unused)]
fn malformed(type_: SignatureToken, data: Vec<u8>) {
    error(type_, data, StatusCode::MALFORMED_CONSTANT_DATA)
}

fn invalid_type(type_: SignatureToken, data: Vec<u8>) {
    error(type_, data, StatusCode::INVALID_CONSTANT_TYPE)
}

fn error(type_: SignatureToken, data: Vec<u8>, code: StatusCode) {
    let mut module = empty_module();
    module.constant_pool = vec![Constant { type_, data }];
    assert!(
        constants::verify_module(&module)
            .unwrap_err()
            .major_status()
            == code
    )
}
