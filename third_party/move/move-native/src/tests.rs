// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use super::*;
use crate::conv::*;
use crate::rt_types::*;
use crate::std::string::*;
use crate::std::vector::{self, *};
use alloc::{string::String, vec, vec::Vec};
use core::mem;
use crate::target_defs::ACCOUNT_ADDRESS_LENGTH;

#[test]
fn test_string_check_utf8() {
    let rust_vec = vec![240, 159, 146, 150];
    let move_vec = rust_vec_to_move_byte_vec(rust_vec);

    let is_utf8 = unsafe { internal_check_utf8(&move_vec) };
    assert!(is_utf8);

    unsafe { move_byte_vec_to_rust_vec(move_vec) };

    let rust_vec = vec![0, 159, 146, 150];
    let move_vec = rust_vec_to_move_byte_vec(rust_vec);

    let is_utf8 = unsafe { internal_check_utf8(&move_vec) };
    assert!(!is_utf8);

    unsafe { move_byte_vec_to_rust_vec(move_vec) };
}

#[test]
fn test_string_is_char_boundary() {
    let rust_vec = String::from("Löwe").into_bytes();
    let move_vec = rust_vec_to_move_byte_vec(rust_vec);

    let is_char_0 = unsafe { internal_is_char_boundary(&move_vec, 0) };
    assert!(is_char_0);

    let is_char_1 = unsafe { internal_is_char_boundary(&move_vec, 2) };
    assert!(!is_char_1);

    unsafe { move_byte_vec_to_rust_vec(move_vec) };
}

#[test]
fn test_sub_string() {
    let rust_vec = b"sub string test".to_vec();
    let move_vec = rust_vec_to_move_byte_vec(rust_vec);

    let move_vec_sub_string = unsafe { internal_sub_string(&move_vec, 0, 10) };
    let rust_vec_sub_string = unsafe { move_byte_vec_to_rust_vec(move_vec_sub_string) };

    assert_eq!(rust_vec_sub_string, b"sub string");

    unsafe { move_byte_vec_to_rust_vec(move_vec) };
}

#[test]
fn test_string_index_of() {
    let rust_vec = b"bears love snow".to_vec();
    let move_vec = rust_vec_to_move_byte_vec(rust_vec);

    let rust_vec_sub = b"love".to_vec();
    let move_vec_sub = rust_vec_to_move_byte_vec(rust_vec_sub);

    let index = unsafe { internal_index_of(&move_vec, &move_vec_sub) };

    assert_eq!(index, 6);

    unsafe { move_byte_vec_to_rust_vec(move_vec) };
    unsafe { move_byte_vec_to_rust_vec(move_vec_sub) };
}

#[test]
fn test_vec_with_bool() {
    static ELEMENT_TYPE: MoveType = MoveType {
        name: DUMMY_TYPE_NAME,
        type_desc: TypeDesc::Bool,
        type_info: &TypeInfo { nothing: 0 },
    };

    let mut move_vec = vector::empty(&ELEMENT_TYPE);
    assert_eq!(move_vec.length, 0);
    assert_eq!(move_vec.capacity, 0);

    let move_vec_len = unsafe { vector::length(&ELEMENT_TYPE, &move_vec) };
    assert_eq!(move_vec_len, 0);

    let mut new_element: bool = true;
    let new_element_ptr = &mut new_element as *mut _ as *mut AnyValue;
    unsafe { vector::push_back(&ELEMENT_TYPE, &mut move_vec, new_element_ptr) }
    assert_eq!(move_vec.length, 1);

    let mut popped_element: bool = false;
    let popped_element_ptr = &mut popped_element as *mut _ as *mut AnyValue;

    unsafe { vector::pop_back(&ELEMENT_TYPE, &mut move_vec, popped_element_ptr) };
    assert_eq!(move_vec.length, 0);
    assert_eq!(popped_element, true);

    unsafe { vector::destroy_empty(&ELEMENT_TYPE, move_vec) }
}

#[test]
fn test_vec_with_vector() {
    static INNER_ELEMENT_TYPE: MoveType = MoveType {
        name: DUMMY_TYPE_NAME,
        type_desc: TypeDesc::Bool,
        type_info: &TypeInfo { nothing: 0 },
    };

    static VECTORTYPEINFO: MoveType = MoveType {
        name: DUMMY_TYPE_NAME,
        type_desc: TypeDesc::Vector,
        type_info: &TypeInfo {
            vector: VectorTypeInfo {
                element_type: &INNER_ELEMENT_TYPE,
            },
        },
    };

    static OUTER_ELEMENT_TYPE: MoveType = MoveType {
        name: DUMMY_TYPE_NAME,
        type_desc: TypeDesc::Vector,
        type_info: &TypeInfo {
            vector: VectorTypeInfo {
                element_type: &VECTORTYPEINFO,
            },
        },
    };

    unsafe {
        let mut move_vec = vector::empty(&OUTER_ELEMENT_TYPE);
        assert_eq!(move_vec.length, 0);
        assert_eq!(move_vec.capacity, 0);

        let move_vec_len = vector::length(&OUTER_ELEMENT_TYPE, &move_vec);
        assert_eq!(move_vec_len, 0);

        let mut new_element_vec = vector::empty(&INNER_ELEMENT_TYPE);

        let mut new_element_inner_0 = true;
        let new_element_inner_ptr_0 = &mut new_element_inner_0 as *mut _ as *mut AnyValue;
        vector::push_back(
            &INNER_ELEMENT_TYPE,
            &mut new_element_vec,
            new_element_inner_ptr_0,
        );

        let mut new_element_inner_1 = false;
        let new_element_inner_ptr_1 = &mut new_element_inner_1 as *mut _ as *mut AnyValue;
        vector::push_back(
            &INNER_ELEMENT_TYPE,
            &mut new_element_vec,
            new_element_inner_ptr_1,
        );

        let new_element_vec_len = vector::length(&INNER_ELEMENT_TYPE, &new_element_vec);
        assert_eq!(new_element_vec_len, 2);

        let new_element_vec_ptr = &mut new_element_vec as *mut _ as *mut AnyValue;
        vector::push_back(&OUTER_ELEMENT_TYPE, &mut move_vec, new_element_vec_ptr);
        assert_eq!(move_vec.length, 1);

        // remove this moved value from current scope
        disarm_drop_bomb(new_element_vec);

        let mut popped_element = vector::empty(&INNER_ELEMENT_TYPE);
        let popped_element_ptr = &mut popped_element as *mut _ as *mut AnyValue;

        vector::pop_back(&OUTER_ELEMENT_TYPE, &mut move_vec, popped_element_ptr);
        assert_eq!(move_vec.length, 0);

        let mut popped_element_inner_0: bool = true;
        let popped_element_inner_ptr_0 = &mut popped_element_inner_0 as *mut _ as *mut AnyValue;
        vector::pop_back(
            &INNER_ELEMENT_TYPE,
            &mut popped_element,
            popped_element_inner_ptr_0,
        );
        assert_eq!(popped_element_inner_0, false);

        let mut popped_element_inner_1: bool = false;
        let popped_element_inner_ptr_1 = &mut popped_element_inner_1 as *mut _ as *mut AnyValue;
        vector::pop_back(
            &INNER_ELEMENT_TYPE,
            &mut popped_element,
            popped_element_inner_ptr_1,
        );
        assert_eq!(popped_element_inner_1, true);

        assert_eq!(popped_element.length, 0);

        vector::destroy_empty(&INNER_ELEMENT_TYPE, popped_element);
        vector::destroy_empty(&OUTER_ELEMENT_TYPE, move_vec);
    }
}

#[test]
fn test_vec_with_signer() {
    static ELEMENT_TYPE: MoveType = MoveType {
        name: DUMMY_TYPE_NAME,
        type_desc: TypeDesc::Signer,
        type_info: &TypeInfo { nothing: 0 },
    };

    let mut move_vec = vector::empty(&ELEMENT_TYPE);
    assert_eq!(move_vec.length, 0);
    assert_eq!(move_vec.capacity, 0);

    let move_vec_len = unsafe { vector::length(&ELEMENT_TYPE, &move_vec) };
    assert_eq!(move_vec_len, 0);

    let mut new_element: MoveSigner = MoveSigner(MoveAddress([u8::MIN; ACCOUNT_ADDRESS_LENGTH]));
    let new_element_ptr = &mut new_element as *mut _ as *mut AnyValue;
    unsafe { vector::push_back(&ELEMENT_TYPE, &mut move_vec, new_element_ptr) }
    assert_eq!(move_vec.length, 1);

    let mut popped_element: MoveSigner = MoveSigner(MoveAddress([u8::MAX; ACCOUNT_ADDRESS_LENGTH]));
    let popped_element_ptr = &mut popped_element as *mut _ as *mut AnyValue;

    unsafe { vector::pop_back(&ELEMENT_TYPE, &mut move_vec, popped_element_ptr) };
    assert_eq!(move_vec.length, 0);
    assert_eq!(popped_element, MoveSigner(MoveAddress([u8::MIN; ACCOUNT_ADDRESS_LENGTH])));

    unsafe { vector::destroy_empty(&ELEMENT_TYPE, move_vec) }
}

#[test]
fn test_vec_with_struct() {
    static STRUCT_FIELD_TYPE: MoveType = MoveType {
        name: DUMMY_TYPE_NAME,
        type_desc: TypeDesc::Bool,
        type_info: &TypeInfo { nothing: 0 },
    };

    static STRUCT_FIELD_INFO: [StructFieldInfo; 2] = [
        StructFieldInfo {
            type_: STRUCT_FIELD_TYPE,
            offset: 0,
        },
        StructFieldInfo {
            type_: STRUCT_FIELD_TYPE,
            offset: 1,
        },
    ];

    static ELEMENT_TYPE: MoveType = MoveType {
        name: DUMMY_TYPE_NAME,
        type_desc: TypeDesc::Struct,
        type_info: &TypeInfo {
            struct_: StructTypeInfo {
                field_array_ptr: &STRUCT_FIELD_INFO[0],
                field_array_len: 2,
                size: mem::size_of::<SimpleStruct>() as u64,
                alignment: mem::align_of::<SimpleStruct>() as u64,
            },
        },
    };

    let mut move_vec = vector::empty(&ELEMENT_TYPE);
    assert_eq!(move_vec.length, 0);
    assert_eq!(move_vec.capacity, 0);

    let move_vec_len = unsafe { vector::length(&ELEMENT_TYPE, &move_vec) };
    assert_eq!(move_vec_len, 0);

    #[repr(C)]
    #[derive(Copy, Clone, Debug, PartialEq)]
    struct SimpleStruct {
        is_black: bool,
        is_white: bool,
    };

    let mut new_element: SimpleStruct = SimpleStruct {
        is_black: true,
        is_white: false,
    };
    let new_element_ptr = &mut new_element as *mut _ as *mut AnyValue;

    unsafe { vector::push_back(&ELEMENT_TYPE, &mut move_vec, new_element_ptr) }
    assert_eq!(move_vec.length, 1);

    let mut popped_element: SimpleStruct = SimpleStruct {
        is_black: false,
        is_white: true,
    };
    let popped_element_ptr = &mut popped_element as *mut _ as *mut AnyValue;

    unsafe { vector::pop_back(&ELEMENT_TYPE, &mut move_vec, popped_element_ptr) };
    assert_eq!(move_vec.length, 0);
    assert_eq!(
        popped_element,
        SimpleStruct {
            is_black: true,
            is_white: false,
        }
    );

    unsafe { vector::destroy_empty(&ELEMENT_TYPE, move_vec) }
}
