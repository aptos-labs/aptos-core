/// Test module for struct and enum transaction arguments in CLI.
///
/// This module contains test types and entry functions for testing the CLI's
/// ability to parse and pass struct/enum arguments to Move entry functions.
module struct_enum_tests::struct_enum_tests {
    use std::option::{Self, Option};
    use std::string::String;
    use std::string::Self;
    use aptos_std::fixed_point32::{Self, FixedPoint32};
    use aptos_std::fixed_point64::{Self, FixedPoint64};
    use std::object::{Self, Object};

    // Test structs for struct transaction arguments

    /// A simple public struct with copy ability for testing struct arguments
    public struct Point has copy, drop {
        x: u64,
        y: u64,
    }

    /// A public struct with nested struct fields
    public struct Rectangle has copy, drop {
        top_left: Point,
        bottom_right: Point,
    }

    /// A public struct with vector field
    public struct Data has copy, drop {
        values: vector<u64>,
        name: String,
    }

    /// A public struct with an enum field, for testing nested-enum-in-struct parsing
    public struct ColoredPoint has copy, drop {
        point: Point,
        color: Color,
    }

    // Test enums for enum transaction arguments

    /// A public enum for testing enum arguments - simple variants
    public enum Color has copy, drop {
        Red,
        Green,
        Blue,
        RGB { r: u8, g: u8, b: u8 },
    }

    /// A public enum with struct-containing variants
    public enum Shape has copy, drop {
        Circle { center: Point, radius: u64 },
        Rectangle { rect: Rectangle },
        Point { point: Point },
    }

    // Test entry functions for struct transaction arguments

    /// Test entry function that takes a Point argument
    public entry fun test_struct_point(_account: &signer, p: Point) {
        // Verify the point values are valid
        assert!(p.x > 0 && p.y > 0, 100);
    }

    /// Test entry function that takes a Rectangle argument
    public entry fun test_struct_rectangle(_account: &signer, rect: Rectangle) {
        // Verify rectangle dimensions are valid
        assert!(rect.bottom_right.x >= rect.top_left.x, 101);
        assert!(rect.bottom_right.y >= rect.top_left.y, 102);
    }

    /// Test entry function that takes Option::Some
    public entry fun test_option_some(_account: &signer, opt: Option<u64>) {
        // Verify it's Some and has expected value
        assert!(option::is_some(&opt), 103);
        let value = option::destroy_some(opt);
        assert!(value == 100, 104);
    }

    /// Test entry function that takes Option::None
    public entry fun test_option_none(_account: &signer, opt: Option<u64>) {
        // Verify it's None
        assert!(option::is_none(&opt), 105);
        option::destroy_none(opt);
    }

    /// Test entry function that takes Option<Point>
    public entry fun test_option_point(_account: &signer, opt: Option<Point>) {
        // Verify it's Some and contains a valid point
        assert!(option::is_some(&opt), 106);
        let point = option::destroy_some(opt);
        assert!(point.x == 50 && point.y == 75, 107);
    }

    /// Test entry function with mixed primitive and struct arguments
    public entry fun test_mixed_args(_account: &signer, num: u64, p: Point, flag: bool) {
        assert!(num == 42, 108);
        assert!(p.x == 10 && p.y == 20, 109);
        assert!(flag == true, 110);
    }

    /// Test entry function with type arguments and struct arguments
    public entry fun test_generic_with_struct<T: drop, U: drop>(
        _account: &signer,
        p: Point,
        value: u64
    ) {
        assert!(p.x == 15 && p.y == 25, 111);
        assert!(value == 999, 112);
    }

    /// Test entry function with Data struct containing vector
    public entry fun test_data_struct(_account: &signer, data: Data) {
        use std::vector;

        // Verify vector contents
        assert!(vector::length(&data.values) == 5, 113);
        assert!(*vector::borrow(&data.values, 0) == 1, 114);
        assert!(*vector::borrow(&data.values, 4) == 5, 115);

        // Verify name
        assert!(string::bytes(&data.name) == &b"test_data", 116);
    }

    // Test entry functions for enum transaction arguments

    /// Test entry function that takes a simple enum variant (no fields)
    public entry fun test_enum_color_simple(_account: &signer, color: Color) {
        // Verify it's the Red variant
        assert!(color is Color::Red, 120);
    }

    /// Test entry function that takes an enum variant with fields
    public entry fun test_enum_color_rgb(_account: &signer, color: Color) {
        // Verify it's the RGB variant with expected values
        assert!(color is Color::RGB, 121);
        match (color) {
            Color::RGB { r, g, b } => {
                assert!(r == 255, 122);
                assert!(g == 128, 123);
                assert!(b == 0, 124);
            },
            _ => abort 125,
        }
    }

    /// Test entry function that takes a Shape enum with Point variant
    public entry fun test_enum_shape_point(_account: &signer, shape: Shape) {
        // Verify it's the Point variant
        assert!(shape is Shape::Point, 126);
        match (shape) {
            Shape::Point { point } => {
                assert!(point.x == 100, 127);
                assert!(point.y == 200, 128);
            },
            _ => abort 129,
        }
    }

    /// Test entry function that takes a Shape enum with Circle variant
    public entry fun test_enum_shape_circle(_account: &signer, shape: Shape) {
        // Verify it's the Circle variant
        assert!(shape is Shape::Circle, 130);
        match (shape) {
            Shape::Circle { center, radius } => {
                assert!(center.x == 50, 131);
                assert!(center.y == 50, 132);
                assert!(radius == 25, 133);
            },
            _ => abort 134,
        }
    }

    /// Test entry function with mixed primitive and enum arguments
    public entry fun test_mixed_with_enum(_account: &signer, num: u64, color: Color, p: Point) {
        assert!(num == 999, 135);
        assert!(color is Color::Green, 136);
        assert!(p.x == 10 && p.y == 20, 137);
    }

    /// Test entry function with Option<Color> enum
    public entry fun test_option_enum(_account: &signer, opt: Option<Color>) {
        // Verify it's Some and contains Green variant
        assert!(option::is_some(&opt), 138);
        let color = option::destroy_some(opt);
        assert!(color is Color::Green, 139);
    }

    // Test entry functions for framework types

    /// Test entry function that takes a String argument
    public entry fun test_framework_string(_account: &signer, s: String) {
        // Verify the string value
        assert!(string::bytes(&s) == &b"hello", 200);
    }

    /// Test entry function that takes FixedPoint32
    public entry fun test_framework_fixed_point32(_account: &signer, fp: FixedPoint32) {
        // Verify the fixed point value (raw value should be 1000)
        assert!(fixed_point32::get_raw_value(fp) == 1000, 201);
    }

    /// Test entry function that takes FixedPoint64
    public entry fun test_framework_fixed_point64(_account: &signer, fp: FixedPoint64) {
        // Verify the fixed point value (raw value should be 2000)
        assert!(fixed_point64::get_raw_value(fp) == 2000, 202);
    }

    /// Test entry function with mixed framework and struct types
    public entry fun test_mixed_framework_struct(
        _account: &signer,
        s: String,
        p: Point,
        fp32: FixedPoint32
    ) {
        assert!(string::bytes(&s) == &b"test", 203);
        assert!(p.x == 100 && p.y == 200, 204);
        assert!(fixed_point32::get_raw_value(fp32) == 500, 205);
    }

    // Test entry functions for vector types with struct elements

    /// Test entry function that takes vector<String>
    public entry fun test_vector_of_strings(_account: &signer, strings: vector<String>) {
        use std::vector;

        // Verify vector length and contents
        assert!(vector::length(&strings) == 3, 210);
        assert!(string::bytes(vector::borrow(&strings, 0)) == &b"hello", 211);
        assert!(string::bytes(vector::borrow(&strings, 1)) == &b"world", 212);
        assert!(string::bytes(vector::borrow(&strings, 2)) == &b"test", 213);
    }

    /// Test entry function that takes vector<Point>
    public entry fun test_vector_of_structs(_account: &signer, points: vector<Point>) {
        use std::vector;

        // Verify vector length
        assert!(vector::length(&points) == 2, 214);

        // Verify first point
        let p1 = vector::borrow(&points, 0);
        assert!(p1.x == 10 && p1.y == 20, 215);

        // Verify second point
        let p2 = vector::borrow(&points, 1);
        assert!(p2.x == 30 && p2.y == 40, 216);
    }

    /// Test entry function that takes vector<Option<u64>>
    public entry fun test_vector_of_options(_account: &signer, opts: vector<Option<u64>>) {
        use std::vector;

        // Verify vector length
        assert!(vector::length(&opts) == 3, 217);

        // First is Some(100)
        let opt1 = vector::borrow(&opts, 0);
        assert!(option::is_some(opt1), 218);
        assert!(*option::borrow(opt1) == 100, 219);

        // Second is None
        let opt2 = vector::borrow(&opts, 1);
        assert!(option::is_none(opt2), 220);

        // Third is Some(200)
        let opt3 = vector::borrow(&opts, 2);
        assert!(option::is_some(opt3), 221);
        assert!(*option::borrow(opt3) == 200, 222);
    }

    /// Test entry function that takes vector<Color> (nested enums in a vector)
    public entry fun test_vector_of_enums(_account: &signer, colors: vector<Color>) {
        use std::vector;

        assert!(vector::length(&colors) == 3, 223);
        let c0 = *vector::borrow(&colors, 0);
        let c1 = *vector::borrow(&colors, 1);
        let c2 = *vector::borrow(&colors, 2);
        assert!(c0 is Color::Red, 224);
        assert!(c1 is Color::Green, 225);
        match (c2) {
            Color::RGB { r, g, b } => {
                assert!(r == 255 && g == 0 && b == 128, 226);
            },
            _ => abort 227,
        }
    }

    /// Test entry function that takes a struct containing an enum field
    public entry fun test_struct_with_enum_field(_account: &signer, cp: ColoredPoint) {
        assert!(cp.point.x == 10 && cp.point.y == 20, 228);
        assert!(cp.color is Color::RGB, 229);
        match (cp.color) {
            Color::RGB { r, g, b } => {
                assert!(r == 255 && g == 0 && b == 128, 230);
            },
            _ => abort 231,
        }
    }
}
