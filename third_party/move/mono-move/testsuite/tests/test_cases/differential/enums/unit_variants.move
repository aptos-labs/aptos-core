// RUN: publish
module 0x42::enums_unit {
    // Unit-only enum: size = 8 (tag only), empty variant offset lists.
    enum Dir has copy, drop {
        North,
        South,
        East,
        West,
    }

    fun code_of(start: u64): u64 {
        let d = if (start == 0) { Dir::North }
            else if (start == 1) { Dir::South }
            else if (start == 2) { Dir::East }
            else { Dir::West };
        if (d is Dir::North) { 100 }
        else if (d is Dir::South) { 200 }
        else if (d is Dir::East) { 300 }
        else { 400 }
    }
}

// RUN: execute 0x42::enums_unit::code_of --args 0
// CHECK: results: 100
// RUN: execute 0x42::enums_unit::code_of --args 1
// CHECK: results: 200
// RUN: execute 0x42::enums_unit::code_of --args 2
// CHECK: results: 300
// RUN: execute 0x42::enums_unit::code_of --args 3
// CHECK: results: 400
