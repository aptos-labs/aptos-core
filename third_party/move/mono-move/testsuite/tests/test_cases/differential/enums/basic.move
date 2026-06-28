// RUN: publish --print(micro-ops)
module 0x42::enums_basic {
    enum Shape has drop {
        Circle { radius: u64 },
        Rectangle { width: u64, height: u64 },
    }

    fun circle_value(r: u64): u64 {
        let s = Shape::Circle { radius: r };
        match (s) {
            Shape::Circle { radius } => radius,
            Shape::Rectangle { width, height } => width + height,
        }
    }

    fun rect_sum(w: u64, h: u64): u64 {
        let s = Shape::Rectangle { width: w, height: h };
        match (s) {
            Shape::Circle { radius } => radius,
            Shape::Rectangle { width, height } => width + height,
        }
    }

    fun is_circle(r: u64): u64 {
        let s = Shape::Circle { radius: r };
        if (s is Shape::Circle) { 1 } else { 0 }
    }

    fun is_rect(w: u64, h: u64): u64 {
        let s = Shape::Rectangle { width: w, height: h };
        if (s is Shape::Rectangle) { 1 } else { 0 }
    }
}

// RUN: execute 0x42::enums_basic::circle_value --args 42
// CHECK: results: 42

// RUN: execute 0x42::enums_basic::rect_sum --args 3, 4
// CHECK: results: 7

// RUN: execute 0x42::enums_basic::is_circle --args 5
// CHECK: results: 1

// RUN: execute 0x42::enums_basic::is_rect --args 3, 4
// CHECK: results: 1
