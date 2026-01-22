/// Module to test public structs and enums with copy ability as transaction arguments
module account::public_struct_test {
    use std::string::String;

    /// Result resource to store test outcomes
    struct TestResult has key {
        value: u64,
        message: String,
    }

    /// A public struct with copy ability - should be allowed as txn arg
    public struct Point has copy, drop {
        x: u64,
        y: u64,
    }

    /// A public struct with nested public copy struct
    public struct Rectangle has copy, drop {
        top_left: Point,
        bottom_right: Point,
    }

    /// A public struct with vector of primitives
    public struct Data has copy, drop {
        values: vector<u64>,
        name: String,
    }

    /// A public enum with copy ability - should be allowed as txn arg
    public enum Color has copy, drop {
        Red,
        Green,
        Blue,
        Custom { r: u8, g: u8, b: u8 },
    }

    /// A public enum with struct fields
    public enum Shape has copy, drop {
        Circle { center: Point, radius: u64 },
        Rect { rect: Rectangle },
    }

    /// A generic container struct - for testing generic structs with enum type arguments
    public struct Container<T> has copy, drop {
        value: T,
    }

    /// A private (non-public) struct with copy ability - should NOT be allowed as txn arg
    struct PrivatePoint has copy, drop {
        x: u64,
        y: u64,
    }

    /// Entry function that takes a Point as argument
    public entry fun test_point(sender: &signer, p: Point) acquires TestResult {
        if (!exists<TestResult>(std::signer::address_of(sender))) {
            move_to(sender, TestResult {
                value: 0,
                message: std::string::utf8(b"")
            });
        };
        let result = borrow_global_mut<TestResult>(std::signer::address_of(sender));
        result.value = p.x + p.y;
        result.message = std::string::utf8(b"point_received");
    }

    /// Entry function that takes a Rectangle as argument
    public entry fun test_rectangle(sender: &signer, r: Rectangle) acquires TestResult {
        if (!exists<TestResult>(std::signer::address_of(sender))) {
            move_to(sender, TestResult {
                value: 0,
                message: std::string::utf8(b"")
            });
        };
        let result = borrow_global_mut<TestResult>(std::signer::address_of(sender));
        result.value = r.top_left.x + r.top_left.y + r.bottom_right.x + r.bottom_right.y;
        result.message = std::string::utf8(b"rectangle_received");
    }

    /// Entry function that takes a Data struct as argument
    public entry fun test_data(sender: &signer, d: Data) acquires TestResult {
        if (!exists<TestResult>(std::signer::address_of(sender))) {
            move_to(sender, TestResult {
                value: 0,
                message: std::string::utf8(b"")
            });
        };
        let result = borrow_global_mut<TestResult>(std::signer::address_of(sender));
        let sum = 0u64;
        let i = 0;
        let len = std::vector::length(&d.values);
        while (i < len) {
            sum = sum + *std::vector::borrow(&d.values, i);
            i = i + 1;
        };
        result.value = sum;
        result.message = d.name;
    }

    /// Entry function that takes a Color enum as argument
    public entry fun test_color(sender: &signer, c: Color) acquires TestResult {
        if (!exists<TestResult>(std::signer::address_of(sender))) {
            move_to(sender, TestResult {
                value: 0,
                message: std::string::utf8(b"")
            });
        };
        let result = borrow_global_mut<TestResult>(std::signer::address_of(sender));
        let (value, msg) = match (c) {
            Color::Red => (1, b"red"),
            Color::Green => (2, b"green"),
            Color::Blue => (3, b"blue"),
            Color::Custom { r, g, b } => ((r as u64) + (g as u64) + (b as u64), b"custom"),
        };
        result.value = value;
        result.message = std::string::utf8(msg);
    }

    /// Entry function that takes a Shape enum as argument
    public entry fun test_shape(sender: &signer, s: Shape) acquires TestResult {
        if (!exists<TestResult>(std::signer::address_of(sender))) {
            move_to(sender, TestResult {
                value: 0,
                message: std::string::utf8(b"")
            });
        };
        let result = borrow_global_mut<TestResult>(std::signer::address_of(sender));
        let (value, msg) = match (s) {
            Shape::Circle { center, radius } => (center.x + center.y + radius, b"circle"),
            Shape::Rect { rect } => (
                rect.top_left.x + rect.top_left.y + rect.bottom_right.x + rect.bottom_right.y,
                b"rect"
            ),
        };
        result.value = value;
        result.message = std::string::utf8(msg);
    }

    /// Entry function that takes a vector of Points
    public entry fun test_point_vector(sender: &signer, points: vector<Point>) acquires TestResult {
        if (!exists<TestResult>(std::signer::address_of(sender))) {
            move_to(sender, TestResult {
                value: 0,
                message: std::string::utf8(b"")
            });
        };
        let result = borrow_global_mut<TestResult>(std::signer::address_of(sender));
        let sum = 0u64;
        let i = 0;
        let len = std::vector::length(&points);
        while (i < len) {
            let p = std::vector::borrow(&points, i);
            sum = sum + p.x + p.y;
            i = i + 1;
        };
        result.value = sum;
        result.message = std::string::utf8(b"point_vector_received");
    }

    /// Entry function using whitelisted String type - should always work
    public entry fun test_string(sender: &signer, s: String) acquires TestResult {
        if (!exists<TestResult>(std::signer::address_of(sender))) {
            move_to(sender, TestResult {
                value: 0,
                message: std::string::utf8(b"")
            });
        };
        let result = borrow_global_mut<TestResult>(std::signer::address_of(sender));
        result.value = std::string::length(&s);
        result.message = s;
    }

    /// Entry function that takes Option<Point>
    public entry fun test_option_point(sender: &signer, opt_point: std::option::Option<Point>) acquires TestResult {
        if (!exists<TestResult>(std::signer::address_of(sender))) {
            move_to(sender, TestResult {
                value: 0,
                message: std::string::utf8(b"")
            });
        };
        let result = borrow_global_mut<TestResult>(std::signer::address_of(sender));
        if (std::option::is_some(&opt_point)) {
            let p = std::option::destroy_some(opt_point);
            result.value = p.x + p.y;
            result.message = std::string::utf8(b"some_point");
        } else {
            std::option::destroy_none(opt_point);
            result.value = 0;
            result.message = std::string::utf8(b"none_point");
        }
    }

    /// Entry function that takes Option<Color>
    public entry fun test_option_color(sender: &signer, opt_color: std::option::Option<Color>) acquires TestResult {
        if (!exists<TestResult>(std::signer::address_of(sender))) {
            move_to(sender, TestResult {
                value: 0,
                message: std::string::utf8(b"")
            });
        };
        let result = borrow_global_mut<TestResult>(std::signer::address_of(sender));
        if (std::option::is_some(&opt_color)) {
            let color = std::option::destroy_some(opt_color);
            let (value, msg) = match (color) {
                Color::Red => (1, b"some_red"),
                Color::Green => (2, b"some_green"),
                Color::Blue => (3, b"some_blue"),
                Color::Custom { r, g, b } => ((r as u64) + (g as u64) + (b as u64), b"some_custom"),
            };
            result.value = value;
            result.message = std::string::utf8(msg);
        } else {
            std::option::destroy_none(opt_color);
            result.value = 0;
            result.message = std::string::utf8(b"none_color");
        }
    }

    /// Entry function that takes Container<Color> as argument
    public entry fun test_container_color(sender: &signer, container: Container<Color>) acquires TestResult {
        if (!exists<TestResult>(std::signer::address_of(sender))) {
            move_to(sender, TestResult {
                value: 0,
                message: std::string::utf8(b"")
            });
        };
        let result = borrow_global_mut<TestResult>(std::signer::address_of(sender));
        let (value, msg) = match (container.value) {
            Color::Red => (100, b"container_red"),
            Color::Green => (200, b"container_green"),
            Color::Blue => (300, b"container_blue"),
            Color::Custom { r, g, b } => ((r as u64) * 100 + (g as u64) * 10 + (b as u64), b"container_custom"),
        };
        result.value = value;
        result.message = std::string::utf8(msg);
    }

    /// Generic entry function that takes Container<T> with any type T that has copy + drop
    /// This is used to test that Container with non-public type arguments is rejected
    public entry fun test_generic_container<T: copy + drop>(sender: &signer, _container: Container<T>) acquires TestResult {
        if (!exists<TestResult>(std::signer::address_of(sender))) {
            move_to(sender, TestResult {
                value: 0,
                message: std::string::utf8(b"")
            });
        };
        let result = borrow_global_mut<TestResult>(std::signer::address_of(sender));
        result.value = 999;
        result.message = std::string::utf8(b"generic_container_received");
    }

    #[view]
    public fun get_result(addr: address): (u64, String) acquires TestResult {
        let result = borrow_global<TestResult>(addr);
        (result.value, result.message)
    }
}
