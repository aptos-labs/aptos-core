// RUN: publish
module 0x42::enums_in_struct {
    enum Tag has drop {
        A,
        B { extra: u64 },
    }

    struct Record has drop {
        tag: Tag,
        payload: u64,
    }

    fun make_and_read(extra: u64, payload: u64): u64 {
        let r = Record { tag: Tag::B { extra }, payload };
        let from_tag = match (&r.tag) {
            Tag::A => 0,
            Tag::B { extra } => *extra,
        };
        from_tag + r.payload
    }
}

// RUN: execute 0x42::enums_in_struct::make_and_read --args 7, 35
// CHECK: results: 42
