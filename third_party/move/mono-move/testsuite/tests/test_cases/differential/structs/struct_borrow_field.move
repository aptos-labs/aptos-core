// RUN: publish
module 0x42::struct_borrow_field {
    struct Entry has drop { key: u64, value: u64 }

    public fun borrow_field(): u64 {
        let e = Entry { key: 5, value: 10 };
        let r = &mut e.value;
        let read = *r;
        *r = 77;
        read * 1000 + e.value
    }
}

// RUN: execute 0x42::struct_borrow_field::borrow_field
// CHECK: results: 10077
