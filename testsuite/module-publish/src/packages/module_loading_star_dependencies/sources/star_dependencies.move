module publisher_address::tree_dependencies {
    use publisher_address::tree_dependencies_1;
    use publisher_address::tree_dependencies_2;
    use publisher_address::tree_dependencies_3;
    use publisher_address::tree_dependencies_4;
    use publisher_address::tree_dependencies_5;
    use publisher_address::tree_dependencies_6;
    use publisher_address::tree_dependencies_7;
    use publisher_address::tree_dependencies_8;
    use publisher_address::tree_dependencies_9;
    use publisher_address::tree_dependencies_10;
    use publisher_address::tree_dependencies_11;
    use publisher_address::tree_dependencies_12;
    use publisher_address::tree_dependencies_13;
    use publisher_address::tree_dependencies_14;
    use publisher_address::tree_dependencies_15;
    use publisher_address::tree_dependencies_16;
    use publisher_address::tree_dependencies_17;
    use publisher_address::tree_dependencies_18;
    use publisher_address::tree_dependencies_19;
    use publisher_address::tree_dependencies_20;

    public entry fun run() {
        let sum = 0;

        sum = sum + tree_dependencies_1::next();
        sum = sum + tree_dependencies_2::next();
        sum = sum + tree_dependencies_3::next();
        sum = sum + tree_dependencies_4::next();
        sum = sum + tree_dependencies_5::next();
        sum = sum + tree_dependencies_6::next();
        sum = sum + tree_dependencies_7::next();
        sum = sum + tree_dependencies_8::next();
        sum = sum + tree_dependencies_9::next();
        sum = sum + tree_dependencies_10::next();
        sum = sum + tree_dependencies_11::next();
        sum = sum + tree_dependencies_12::next();
        sum = sum + tree_dependencies_13::next();
        sum = sum + tree_dependencies_14::next();
        sum = sum + tree_dependencies_15::next();
        sum = sum + tree_dependencies_16::next();
        sum = sum + tree_dependencies_17::next();
        sum = sum + tree_dependencies_18::next();
        sum = sum + tree_dependencies_19::next();
        sum = sum + tree_dependencies_20::next();

        assert!(sum == 20, 77);
    }
}

module publisher_address::tree_dependencies_1 {

    const MAGIC: u64 = 1;

    public fun next(): u64 {
        1
    }

    // Functions bellow are used to make module verification a bit more expensive.

    struct Data has copy, drop, store {
        data: vector<u8>,
    }

    struct Resource has key {
        id: u64,
        name: aptos_std::string::String,
        data: Data,
    }

    struct Counter has key {
        count: u64,
    }

    public fun copy_pasta_ref(
        r1: &Resource,
        r2: &Resource,
        c1: &Counter,
        c2: &Counter,
    ): &u64 {
        let ret1 = &r1.id;
        let ret2 = &r2.id;
        if (*ret1 < *ret2) {
            ret1 = ret2;
            ret2 = &c1.count;
        } else {
            ret1 = &r2.id;
            ret2 = &c2.count;
        };
        if (*ret2 < r2.id) {
            ret1 = ret2;
            ret2 = &c2.count;
        } else if (ret1 != &r1.id) {
            ret1 = &c1.count;
            ret2 = &r2.id;
        };
        if (*ret1 < *ret2) {
            ret2 = ret1;
            ret1
        } else {
            ret1 = ret2;
            ret2
        };
        if (ret1 == ret2) {
            ret1
        } else {
            ret2
        }
    }

    public fun loop_bcs(count: u64, len: u64) {
        let vec = std::vector::empty<u64>();
        let i = 0;
        while (i < len) {
            std::vector::push_back(&mut vec, i);
            i = i + 1;
        };

        let sum: u64 = 0;

        while (count > 0) {
            let val = std::bcs::to_bytes(&vec);
            sum = sum + ((*std::vector::borrow(&val, 0)) as u64);
            count = count - 1;
        }
    }
}

module publisher_address::tree_dependencies_2 {

    const MAGIC: u64 = 2;

    public fun next(): u64 {
        1
    }

    // Functions bellow are used to make module verification a bit more expensive.

    struct Data has copy, drop, store {
        data: vector<u8>,
    }

    struct Resource has key {
        id: u64,
        name: aptos_std::string::String,
        data: Data,
    }

    struct Counter has key {
        count: u64,
    }

    public fun copy_pasta_ref(
        r1: &Resource,
        r2: &Resource,
        c1: &Counter,
        c2: &Counter,
    ): &u64 {
        let ret1 = &r1.id;
        let ret2 = &r2.id;
        if (*ret1 < *ret2) {
            ret1 = ret2;
            ret2 = &c1.count;
        } else {
            ret1 = &r2.id;
            ret2 = &c2.count;
        };
        if (*ret2 < r2.id) {
            ret1 = ret2;
            ret2 = &c2.count;
        } else if (ret1 != &r1.id) {
            ret1 = &c1.count;
            ret2 = &r2.id;
        };
        if (*ret1 < *ret2) {
            ret2 = ret1;
            ret1
        } else {
            ret1 = ret2;
            ret2
        };
        if (ret1 == ret2) {
            ret1
        } else {
            ret2
        }
    }

    public fun loop_bcs(count: u64, len: u64) {
        let vec = std::vector::empty<u64>();
        let i = 0;
        while (i < len) {
            std::vector::push_back(&mut vec, i);
            i = i + 1;
        };

        let sum: u64 = 0;

        while (count > 0) {
            let val = std::bcs::to_bytes(&vec);
            sum = sum + ((*std::vector::borrow(&val, 0)) as u64);
            count = count - 1;
        }
    }
}

module publisher_address::tree_dependencies_3 {

    const MAGIC: u64 = 3;

    public fun next(): u64 {
        1
    }

    // Functions bellow are used to make module verification a bit more expensive.

    struct Data has copy, drop, store {
        data: vector<u8>,
    }

    struct Resource has key {
        id: u64,
        name: aptos_std::string::String,
        data: Data,
    }

    struct Counter has key {
        count: u64,
    }

    public fun copy_pasta_ref(
        r1: &Resource,
        r2: &Resource,
        c1: &Counter,
        c2: &Counter,
    ): &u64 {
        let ret1 = &r1.id;
        let ret2 = &r2.id;
        if (*ret1 < *ret2) {
            ret1 = ret2;
            ret2 = &c1.count;
        } else {
            ret1 = &r2.id;
            ret2 = &c2.count;
        };
        if (*ret2 < r2.id) {
            ret1 = ret2;
            ret2 = &c2.count;
        } else if (ret1 != &r1.id) {
            ret1 = &c1.count;
            ret2 = &r2.id;
        };
        if (*ret1 < *ret2) {
            ret2 = ret1;
            ret1
        } else {
            ret1 = ret2;
            ret2
        };
        if (ret1 == ret2) {
            ret1
        } else {
            ret2
        }
    }

    public fun loop_bcs(count: u64, len: u64) {
        let vec = std::vector::empty<u64>();
        let i = 0;
        while (i < len) {
            std::vector::push_back(&mut vec, i);
            i = i + 1;
        };

        let sum: u64 = 0;

        while (count > 0) {
            let val = std::bcs::to_bytes(&vec);
            sum = sum + ((*std::vector::borrow(&val, 0)) as u64);
            count = count - 1;
        }
    }
}

module publisher_address::tree_dependencies_4 {

    const MAGIC: u64 = 4;

    public fun next(): u64 {
        1
    }

    // Functions bellow are used to make module verification a bit more expensive.

    struct Data has copy, drop, store {
        data: vector<u8>,
    }

    struct Resource has key {
        id: u64,
        name: aptos_std::string::String,
        data: Data,
    }

    struct Counter has key {
        count: u64,
    }

    public fun copy_pasta_ref(
        r1: &Resource,
        r2: &Resource,
        c1: &Counter,
        c2: &Counter,
    ): &u64 {
        let ret1 = &r1.id;
        let ret2 = &r2.id;
        if (*ret1 < *ret2) {
            ret1 = ret2;
            ret2 = &c1.count;
        } else {
            ret1 = &r2.id;
            ret2 = &c2.count;
        };
        if (*ret2 < r2.id) {
            ret1 = ret2;
            ret2 = &c2.count;
        } else if (ret1 != &r1.id) {
            ret1 = &c1.count;
            ret2 = &r2.id;
        };
        if (*ret1 < *ret2) {
            ret2 = ret1;
            ret1
        } else {
            ret1 = ret2;
            ret2
        };
        if (ret1 == ret2) {
            ret1
        } else {
            ret2
        }
    }

    public fun loop_bcs(count: u64, len: u64) {
        let vec = std::vector::empty<u64>();
        let i = 0;
        while (i < len) {
            std::vector::push_back(&mut vec, i);
            i = i + 1;
        };

        let sum: u64 = 0;

        while (count > 0) {
            let val = std::bcs::to_bytes(&vec);
            sum = sum + ((*std::vector::borrow(&val, 0)) as u64);
            count = count - 1;
        }
    }
}

module publisher_address::tree_dependencies_5 {

    const MAGIC: u64 = 5;

    public fun next(): u64 {
        1
    }

    // Functions bellow are used to make module verification a bit more expensive.

    struct Data has copy, drop, store {
        data: vector<u8>,
    }

    struct Resource has key {
        id: u64,
        name: aptos_std::string::String,
        data: Data,
    }

    struct Counter has key {
        count: u64,
    }

    public fun copy_pasta_ref(
        r1: &Resource,
        r2: &Resource,
        c1: &Counter,
        c2: &Counter,
    ): &u64 {
        let ret1 = &r1.id;
        let ret2 = &r2.id;
        if (*ret1 < *ret2) {
            ret1 = ret2;
            ret2 = &c1.count;
        } else {
            ret1 = &r2.id;
            ret2 = &c2.count;
        };
        if (*ret2 < r2.id) {
            ret1 = ret2;
            ret2 = &c2.count;
        } else if (ret1 != &r1.id) {
            ret1 = &c1.count;
            ret2 = &r2.id;
        };
        if (*ret1 < *ret2) {
            ret2 = ret1;
            ret1
        } else {
            ret1 = ret2;
            ret2
        };
        if (ret1 == ret2) {
            ret1
        } else {
            ret2
        }
    }

    public fun loop_bcs(count: u64, len: u64) {
        let vec = std::vector::empty<u64>();
        let i = 0;
        while (i < len) {
            std::vector::push_back(&mut vec, i);
            i = i + 1;
        };

        let sum: u64 = 0;

        while (count > 0) {
            let val = std::bcs::to_bytes(&vec);
            sum = sum + ((*std::vector::borrow(&val, 0)) as u64);
            count = count - 1;
        }
    }
}

module publisher_address::tree_dependencies_6 {

    const MAGIC: u64 = 6;

    public fun next(): u64 {
        1
    }

    // Functions bellow are used to make module verification a bit more expensive.

    struct Data has copy, drop, store {
        data: vector<u8>,
    }

    struct Resource has key {
        id: u64,
        name: aptos_std::string::String,
        data: Data,
    }

    struct Counter has key {
        count: u64,
    }

    public fun copy_pasta_ref(
        r1: &Resource,
        r2: &Resource,
        c1: &Counter,
        c2: &Counter,
    ): &u64 {
        let ret1 = &r1.id;
        let ret2 = &r2.id;
        if (*ret1 < *ret2) {
            ret1 = ret2;
            ret2 = &c1.count;
        } else {
            ret1 = &r2.id;
            ret2 = &c2.count;
        };
        if (*ret2 < r2.id) {
            ret1 = ret2;
            ret2 = &c2.count;
        } else if (ret1 != &r1.id) {
            ret1 = &c1.count;
            ret2 = &r2.id;
        };
        if (*ret1 < *ret2) {
            ret2 = ret1;
            ret1
        } else {
            ret1 = ret2;
            ret2
        };
        if (ret1 == ret2) {
            ret1
        } else {
            ret2
        }
    }

    public fun loop_bcs(count: u64, len: u64) {
        let vec = std::vector::empty<u64>();
        let i = 0;
        while (i < len) {
            std::vector::push_back(&mut vec, i);
            i = i + 1;
        };

        let sum: u64 = 0;

        while (count > 0) {
            let val = std::bcs::to_bytes(&vec);
            sum = sum + ((*std::vector::borrow(&val, 0)) as u64);
            count = count - 1;
        }
    }
}

module publisher_address::tree_dependencies_7 {

    const MAGIC: u64 = 7;

    public fun next(): u64 {
        1
    }

    // Functions bellow are used to make module verification a bit more expensive.

    struct Data has copy, drop, store {
        data: vector<u8>,
    }

    struct Resource has key {
        id: u64,
        name: aptos_std::string::String,
        data: Data,
    }

    struct Counter has key {
        count: u64,
    }

    public fun copy_pasta_ref(
        r1: &Resource,
        r2: &Resource,
        c1: &Counter,
        c2: &Counter,
    ): &u64 {
        let ret1 = &r1.id;
        let ret2 = &r2.id;
        if (*ret1 < *ret2) {
            ret1 = ret2;
            ret2 = &c1.count;
        } else {
            ret1 = &r2.id;
            ret2 = &c2.count;
        };
        if (*ret2 < r2.id) {
            ret1 = ret2;
            ret2 = &c2.count;
        } else if (ret1 != &r1.id) {
            ret1 = &c1.count;
            ret2 = &r2.id;
        };
        if (*ret1 < *ret2) {
            ret2 = ret1;
            ret1
        } else {
            ret1 = ret2;
            ret2
        };
        if (ret1 == ret2) {
            ret1
        } else {
            ret2
        }
    }

    public fun loop_bcs(count: u64, len: u64) {
        let vec = std::vector::empty<u64>();
        let i = 0;
        while (i < len) {
            std::vector::push_back(&mut vec, i);
            i = i + 1;
        };

        let sum: u64 = 0;

        while (count > 0) {
            let val = std::bcs::to_bytes(&vec);
            sum = sum + ((*std::vector::borrow(&val, 0)) as u64);
            count = count - 1;
        }
    }
}

module publisher_address::tree_dependencies_8 {

    const MAGIC: u64 = 8;

    public fun next(): u64 {
        1
    }

    // Functions bellow are used to make module verification a bit more expensive.

    struct Data has copy, drop, store {
        data: vector<u8>,
    }

    struct Resource has key {
        id: u64,
        name: aptos_std::string::String,
        data: Data,
    }

    struct Counter has key {
        count: u64,
    }

    public fun copy_pasta_ref(
        r1: &Resource,
        r2: &Resource,
        c1: &Counter,
        c2: &Counter,
    ): &u64 {
        let ret1 = &r1.id;
        let ret2 = &r2.id;
        if (*ret1 < *ret2) {
            ret1 = ret2;
            ret2 = &c1.count;
        } else {
            ret1 = &r2.id;
            ret2 = &c2.count;
        };
        if (*ret2 < r2.id) {
            ret1 = ret2;
            ret2 = &c2.count;
        } else if (ret1 != &r1.id) {
            ret1 = &c1.count;
            ret2 = &r2.id;
        };
        if (*ret1 < *ret2) {
            ret2 = ret1;
            ret1
        } else {
            ret1 = ret2;
            ret2
        };
        if (ret1 == ret2) {
            ret1
        } else {
            ret2
        }
    }

    public fun loop_bcs(count: u64, len: u64) {
        let vec = std::vector::empty<u64>();
        let i = 0;
        while (i < len) {
            std::vector::push_back(&mut vec, i);
            i = i + 1;
        };

        let sum: u64 = 0;

        while (count > 0) {
            let val = std::bcs::to_bytes(&vec);
            sum = sum + ((*std::vector::borrow(&val, 0)) as u64);
            count = count - 1;
        }
    }
}

module publisher_address::tree_dependencies_9 {

    const MAGIC: u64 = 9;

    public fun next(): u64 {
        1
    }

    // Functions bellow are used to make module verification a bit more expensive.

    struct Data has copy, drop, store {
        data: vector<u8>,
    }

    struct Resource has key {
        id: u64,
        name: aptos_std::string::String,
        data: Data,
    }

    struct Counter has key {
        count: u64,
    }

    public fun copy_pasta_ref(
        r1: &Resource,
        r2: &Resource,
        c1: &Counter,
        c2: &Counter,
    ): &u64 {
        let ret1 = &r1.id;
        let ret2 = &r2.id;
        if (*ret1 < *ret2) {
            ret1 = ret2;
            ret2 = &c1.count;
        } else {
            ret1 = &r2.id;
            ret2 = &c2.count;
        };
        if (*ret2 < r2.id) {
            ret1 = ret2;
            ret2 = &c2.count;
        } else if (ret1 != &r1.id) {
            ret1 = &c1.count;
            ret2 = &r2.id;
        };
        if (*ret1 < *ret2) {
            ret2 = ret1;
            ret1
        } else {
            ret1 = ret2;
            ret2
        };
        if (ret1 == ret2) {
            ret1
        } else {
            ret2
        }
    }

    public fun loop_bcs(count: u64, len: u64) {
        let vec = std::vector::empty<u64>();
        let i = 0;
        while (i < len) {
            std::vector::push_back(&mut vec, i);
            i = i + 1;
        };

        let sum: u64 = 0;

        while (count > 0) {
            let val = std::bcs::to_bytes(&vec);
            sum = sum + ((*std::vector::borrow(&val, 0)) as u64);
            count = count - 1;
        }
    }
}

module publisher_address::tree_dependencies_10 {

    const MAGIC: u64 = 10;

    public fun next(): u64 {
        1
    }

    // Functions bellow are used to make module verification a bit more expensive.

    struct Data has copy, drop, store {
        data: vector<u8>,
    }

    struct Resource has key {
        id: u64,
        name: aptos_std::string::String,
        data: Data,
    }

    struct Counter has key {
        count: u64,
    }

    public fun copy_pasta_ref(
        r1: &Resource,
        r2: &Resource,
        c1: &Counter,
        c2: &Counter,
    ): &u64 {
        let ret1 = &r1.id;
        let ret2 = &r2.id;
        if (*ret1 < *ret2) {
            ret1 = ret2;
            ret2 = &c1.count;
        } else {
            ret1 = &r2.id;
            ret2 = &c2.count;
        };
        if (*ret2 < r2.id) {
            ret1 = ret2;
            ret2 = &c2.count;
        } else if (ret1 != &r1.id) {
            ret1 = &c1.count;
            ret2 = &r2.id;
        };
        if (*ret1 < *ret2) {
            ret2 = ret1;
            ret1
        } else {
            ret1 = ret2;
            ret2
        };
        if (ret1 == ret2) {
            ret1
        } else {
            ret2
        }
    }

    public fun loop_bcs(count: u64, len: u64) {
        let vec = std::vector::empty<u64>();
        let i = 0;
        while (i < len) {
            std::vector::push_back(&mut vec, i);
            i = i + 1;
        };

        let sum: u64 = 0;

        while (count > 0) {
            let val = std::bcs::to_bytes(&vec);
            sum = sum + ((*std::vector::borrow(&val, 0)) as u64);
            count = count - 1;
        }
    }
}

module publisher_address::tree_dependencies_11 {

    const MAGIC: u64 = 11;

    public fun next(): u64 {
        1
    }

    // Functions bellow are used to make module verification a bit more expensive.

    struct Data has copy, drop, store {
        data: vector<u8>,
    }

    struct Resource has key {
        id: u64,
        name: aptos_std::string::String,
        data: Data,
    }

    struct Counter has key {
        count: u64,
    }

    public fun copy_pasta_ref(
        r1: &Resource,
        r2: &Resource,
        c1: &Counter,
        c2: &Counter,
    ): &u64 {
        let ret1 = &r1.id;
        let ret2 = &r2.id;
        if (*ret1 < *ret2) {
            ret1 = ret2;
            ret2 = &c1.count;
        } else {
            ret1 = &r2.id;
            ret2 = &c2.count;
        };
        if (*ret2 < r2.id) {
            ret1 = ret2;
            ret2 = &c2.count;
        } else if (ret1 != &r1.id) {
            ret1 = &c1.count;
            ret2 = &r2.id;
        };
        if (*ret1 < *ret2) {
            ret2 = ret1;
            ret1
        } else {
            ret1 = ret2;
            ret2
        };
        if (ret1 == ret2) {
            ret1
        } else {
            ret2
        }
    }

    public fun loop_bcs(count: u64, len: u64) {
        let vec = std::vector::empty<u64>();
        let i = 0;
        while (i < len) {
            std::vector::push_back(&mut vec, i);
            i = i + 1;
        };

        let sum: u64 = 0;

        while (count > 0) {
            let val = std::bcs::to_bytes(&vec);
            sum = sum + ((*std::vector::borrow(&val, 0)) as u64);
            count = count - 1;
        }
    }
}

module publisher_address::tree_dependencies_12 {

    const MAGIC: u64 = 12;

    public fun next(): u64 {
        1
    }

    // Functions bellow are used to make module verification a bit more expensive.

    struct Data has copy, drop, store {
        data: vector<u8>,
    }

    struct Resource has key {
        id: u64,
        name: aptos_std::string::String,
        data: Data,
    }

    struct Counter has key {
        count: u64,
    }

    public fun copy_pasta_ref(
        r1: &Resource,
        r2: &Resource,
        c1: &Counter,
        c2: &Counter,
    ): &u64 {
        let ret1 = &r1.id;
        let ret2 = &r2.id;
        if (*ret1 < *ret2) {
            ret1 = ret2;
            ret2 = &c1.count;
        } else {
            ret1 = &r2.id;
            ret2 = &c2.count;
        };
        if (*ret2 < r2.id) {
            ret1 = ret2;
            ret2 = &c2.count;
        } else if (ret1 != &r1.id) {
            ret1 = &c1.count;
            ret2 = &r2.id;
        };
        if (*ret1 < *ret2) {
            ret2 = ret1;
            ret1
        } else {
            ret1 = ret2;
            ret2
        };
        if (ret1 == ret2) {
            ret1
        } else {
            ret2
        }
    }

    public fun loop_bcs(count: u64, len: u64) {
        let vec = std::vector::empty<u64>();
        let i = 0;
        while (i < len) {
            std::vector::push_back(&mut vec, i);
            i = i + 1;
        };

        let sum: u64 = 0;

        while (count > 0) {
            let val = std::bcs::to_bytes(&vec);
            sum = sum + ((*std::vector::borrow(&val, 0)) as u64);
            count = count - 1;
        }
    }
}

module publisher_address::tree_dependencies_13 {

    const MAGIC: u64 = 13;

    public fun next(): u64 {
        1
    }

    // Functions bellow are used to make module verification a bit more expensive.

    struct Data has copy, drop, store {
        data: vector<u8>,
    }

    struct Resource has key {
        id: u64,
        name: aptos_std::string::String,
        data: Data,
    }

    struct Counter has key {
        count: u64,
    }

    public fun copy_pasta_ref(
        r1: &Resource,
        r2: &Resource,
        c1: &Counter,
        c2: &Counter,
    ): &u64 {
        let ret1 = &r1.id;
        let ret2 = &r2.id;
        if (*ret1 < *ret2) {
            ret1 = ret2;
            ret2 = &c1.count;
        } else {
            ret1 = &r2.id;
            ret2 = &c2.count;
        };
        if (*ret2 < r2.id) {
            ret1 = ret2;
            ret2 = &c2.count;
        } else if (ret1 != &r1.id) {
            ret1 = &c1.count;
            ret2 = &r2.id;
        };
        if (*ret1 < *ret2) {
            ret2 = ret1;
            ret1
        } else {
            ret1 = ret2;
            ret2
        };
        if (ret1 == ret2) {
            ret1
        } else {
            ret2
        }
    }

    public fun loop_bcs(count: u64, len: u64) {
        let vec = std::vector::empty<u64>();
        let i = 0;
        while (i < len) {
            std::vector::push_back(&mut vec, i);
            i = i + 1;
        };

        let sum: u64 = 0;

        while (count > 0) {
            let val = std::bcs::to_bytes(&vec);
            sum = sum + ((*std::vector::borrow(&val, 0)) as u64);
            count = count - 1;
        }
    }
}

module publisher_address::tree_dependencies_14 {

    const MAGIC: u64 = 14;

    public fun next(): u64 {
        1
    }

    // Functions bellow are used to make module verification a bit more expensive.

    struct Data has copy, drop, store {
        data: vector<u8>,
    }

    struct Resource has key {
        id: u64,
        name: aptos_std::string::String,
        data: Data,
    }

    struct Counter has key {
        count: u64,
    }

    public fun copy_pasta_ref(
        r1: &Resource,
        r2: &Resource,
        c1: &Counter,
        c2: &Counter,
    ): &u64 {
        let ret1 = &r1.id;
        let ret2 = &r2.id;
        if (*ret1 < *ret2) {
            ret1 = ret2;
            ret2 = &c1.count;
        } else {
            ret1 = &r2.id;
            ret2 = &c2.count;
        };
        if (*ret2 < r2.id) {
            ret1 = ret2;
            ret2 = &c2.count;
        } else if (ret1 != &r1.id) {
            ret1 = &c1.count;
            ret2 = &r2.id;
        };
        if (*ret1 < *ret2) {
            ret2 = ret1;
            ret1
        } else {
            ret1 = ret2;
            ret2
        };
        if (ret1 == ret2) {
            ret1
        } else {
            ret2
        }
    }

    public fun loop_bcs(count: u64, len: u64) {
        let vec = std::vector::empty<u64>();
        let i = 0;
        while (i < len) {
            std::vector::push_back(&mut vec, i);
            i = i + 1;
        };

        let sum: u64 = 0;

        while (count > 0) {
            let val = std::bcs::to_bytes(&vec);
            sum = sum + ((*std::vector::borrow(&val, 0)) as u64);
            count = count - 1;
        }
    }
}

module publisher_address::tree_dependencies_15 {

    const MAGIC: u64 = 15;

    public fun next(): u64 {
        1
    }

    // Functions bellow are used to make module verification a bit more expensive.

    struct Data has copy, drop, store {
        data: vector<u8>,
    }

    struct Resource has key {
        id: u64,
        name: aptos_std::string::String,
        data: Data,
    }

    struct Counter has key {
        count: u64,
    }

    public fun copy_pasta_ref(
        r1: &Resource,
        r2: &Resource,
        c1: &Counter,
        c2: &Counter,
    ): &u64 {
        let ret1 = &r1.id;
        let ret2 = &r2.id;
        if (*ret1 < *ret2) {
            ret1 = ret2;
            ret2 = &c1.count;
        } else {
            ret1 = &r2.id;
            ret2 = &c2.count;
        };
        if (*ret2 < r2.id) {
            ret1 = ret2;
            ret2 = &c2.count;
        } else if (ret1 != &r1.id) {
            ret1 = &c1.count;
            ret2 = &r2.id;
        };
        if (*ret1 < *ret2) {
            ret2 = ret1;
            ret1
        } else {
            ret1 = ret2;
            ret2
        };
        if (ret1 == ret2) {
            ret1
        } else {
            ret2
        }
    }

    public fun loop_bcs(count: u64, len: u64) {
        let vec = std::vector::empty<u64>();
        let i = 0;
        while (i < len) {
            std::vector::push_back(&mut vec, i);
            i = i + 1;
        };

        let sum: u64 = 0;

        while (count > 0) {
            let val = std::bcs::to_bytes(&vec);
            sum = sum + ((*std::vector::borrow(&val, 0)) as u64);
            count = count - 1;
        }
    }
}

module publisher_address::tree_dependencies_16 {

    const MAGIC: u64 = 16;

    public fun next(): u64 {
        1
    }

    // Functions bellow are used to make module verification a bit more expensive.

    struct Data has copy, drop, store {
        data: vector<u8>,
    }

    struct Resource has key {
        id: u64,
        name: aptos_std::string::String,
        data: Data,
    }

    struct Counter has key {
        count: u64,
    }

    public fun copy_pasta_ref(
        r1: &Resource,
        r2: &Resource,
        c1: &Counter,
        c2: &Counter,
    ): &u64 {
        let ret1 = &r1.id;
        let ret2 = &r2.id;
        if (*ret1 < *ret2) {
            ret1 = ret2;
            ret2 = &c1.count;
        } else {
            ret1 = &r2.id;
            ret2 = &c2.count;
        };
        if (*ret2 < r2.id) {
            ret1 = ret2;
            ret2 = &c2.count;
        } else if (ret1 != &r1.id) {
            ret1 = &c1.count;
            ret2 = &r2.id;
        };
        if (*ret1 < *ret2) {
            ret2 = ret1;
            ret1
        } else {
            ret1 = ret2;
            ret2
        };
        if (ret1 == ret2) {
            ret1
        } else {
            ret2
        }
    }

    public fun loop_bcs(count: u64, len: u64) {
        let vec = std::vector::empty<u64>();
        let i = 0;
        while (i < len) {
            std::vector::push_back(&mut vec, i);
            i = i + 1;
        };

        let sum: u64 = 0;

        while (count > 0) {
            let val = std::bcs::to_bytes(&vec);
            sum = sum + ((*std::vector::borrow(&val, 0)) as u64);
            count = count - 1;
        }
    }
}

module publisher_address::tree_dependencies_17 {

    const MAGIC: u64 = 17;

    public fun next(): u64 {
        1
    }

    // Functions bellow are used to make module verification a bit more expensive.

    struct Data has copy, drop, store {
        data: vector<u8>,
    }

    struct Resource has key {
        id: u64,
        name: aptos_std::string::String,
        data: Data,
    }

    struct Counter has key {
        count: u64,
    }

    public fun copy_pasta_ref(
        r1: &Resource,
        r2: &Resource,
        c1: &Counter,
        c2: &Counter,
    ): &u64 {
        let ret1 = &r1.id;
        let ret2 = &r2.id;
        if (*ret1 < *ret2) {
            ret1 = ret2;
            ret2 = &c1.count;
        } else {
            ret1 = &r2.id;
            ret2 = &c2.count;
        };
        if (*ret2 < r2.id) {
            ret1 = ret2;
            ret2 = &c2.count;
        } else if (ret1 != &r1.id) {
            ret1 = &c1.count;
            ret2 = &r2.id;
        };
        if (*ret1 < *ret2) {
            ret2 = ret1;
            ret1
        } else {
            ret1 = ret2;
            ret2
        };
        if (ret1 == ret2) {
            ret1
        } else {
            ret2
        }
    }

    public fun loop_bcs(count: u64, len: u64) {
        let vec = std::vector::empty<u64>();
        let i = 0;
        while (i < len) {
            std::vector::push_back(&mut vec, i);
            i = i + 1;
        };

        let sum: u64 = 0;

        while (count > 0) {
            let val = std::bcs::to_bytes(&vec);
            sum = sum + ((*std::vector::borrow(&val, 0)) as u64);
            count = count - 1;
        }
    }
}

module publisher_address::tree_dependencies_18 {

    const MAGIC: u64 = 18;

    public fun next(): u64 {
        1
    }

    // Functions bellow are used to make module verification a bit more expensive.

    struct Data has copy, drop, store {
        data: vector<u8>,
    }

    struct Resource has key {
        id: u64,
        name: aptos_std::string::String,
        data: Data,
    }

    struct Counter has key {
        count: u64,
    }

    public fun copy_pasta_ref(
        r1: &Resource,
        r2: &Resource,
        c1: &Counter,
        c2: &Counter,
    ): &u64 {
        let ret1 = &r1.id;
        let ret2 = &r2.id;
        if (*ret1 < *ret2) {
            ret1 = ret2;
            ret2 = &c1.count;
        } else {
            ret1 = &r2.id;
            ret2 = &c2.count;
        };
        if (*ret2 < r2.id) {
            ret1 = ret2;
            ret2 = &c2.count;
        } else if (ret1 != &r1.id) {
            ret1 = &c1.count;
            ret2 = &r2.id;
        };
        if (*ret1 < *ret2) {
            ret2 = ret1;
            ret1
        } else {
            ret1 = ret2;
            ret2
        };
        if (ret1 == ret2) {
            ret1
        } else {
            ret2
        }
    }

    public fun loop_bcs(count: u64, len: u64) {
        let vec = std::vector::empty<u64>();
        let i = 0;
        while (i < len) {
            std::vector::push_back(&mut vec, i);
            i = i + 1;
        };

        let sum: u64 = 0;

        while (count > 0) {
            let val = std::bcs::to_bytes(&vec);
            sum = sum + ((*std::vector::borrow(&val, 0)) as u64);
            count = count - 1;
        }
    }
}

module publisher_address::tree_dependencies_19 {

    const MAGIC: u64 = 19;

    public fun next(): u64 {
        1
    }

    // Functions bellow are used to make module verification a bit more expensive.

    struct Data has copy, drop, store {
        data: vector<u8>,
    }

    struct Resource has key {
        id: u64,
        name: aptos_std::string::String,
        data: Data,
    }

    struct Counter has key {
        count: u64,
    }

    public fun copy_pasta_ref(
        r1: &Resource,
        r2: &Resource,
        c1: &Counter,
        c2: &Counter,
    ): &u64 {
        let ret1 = &r1.id;
        let ret2 = &r2.id;
        if (*ret1 < *ret2) {
            ret1 = ret2;
            ret2 = &c1.count;
        } else {
            ret1 = &r2.id;
            ret2 = &c2.count;
        };
        if (*ret2 < r2.id) {
            ret1 = ret2;
            ret2 = &c2.count;
        } else if (ret1 != &r1.id) {
            ret1 = &c1.count;
            ret2 = &r2.id;
        };
        if (*ret1 < *ret2) {
            ret2 = ret1;
            ret1
        } else {
            ret1 = ret2;
            ret2
        };
        if (ret1 == ret2) {
            ret1
        } else {
            ret2
        }
    }

    public fun loop_bcs(count: u64, len: u64) {
        let vec = std::vector::empty<u64>();
        let i = 0;
        while (i < len) {
            std::vector::push_back(&mut vec, i);
            i = i + 1;
        };

        let sum: u64 = 0;

        while (count > 0) {
            let val = std::bcs::to_bytes(&vec);
            sum = sum + ((*std::vector::borrow(&val, 0)) as u64);
            count = count - 1;
        }
    }
}

module publisher_address::tree_dependencies_20 {

    const MAGIC: u64 = 20;

    public fun next(): u64 {
        1
    }

    // Functions bellow are used to make module verification a bit more expensive.

    struct Data has copy, drop, store {
        data: vector<u8>,
    }

    struct Resource has key {
        id: u64,
        name: aptos_std::string::String,
        data: Data,
    }

    struct Counter has key {
        count: u64,
    }

    public fun copy_pasta_ref(
        r1: &Resource,
        r2: &Resource,
        c1: &Counter,
        c2: &Counter,
    ): &u64 {
        let ret1 = &r1.id;
        let ret2 = &r2.id;
        if (*ret1 < *ret2) {
            ret1 = ret2;
            ret2 = &c1.count;
        } else {
            ret1 = &r2.id;
            ret2 = &c2.count;
        };
        if (*ret2 < r2.id) {
            ret1 = ret2;
            ret2 = &c2.count;
        } else if (ret1 != &r1.id) {
            ret1 = &c1.count;
            ret2 = &r2.id;
        };
        if (*ret1 < *ret2) {
            ret2 = ret1;
            ret1
        } else {
            ret1 = ret2;
            ret2
        };
        if (ret1 == ret2) {
            ret1
        } else {
            ret2
        }
    }

    public fun loop_bcs(count: u64, len: u64) {
        let vec = std::vector::empty<u64>();
        let i = 0;
        while (i < len) {
            std::vector::push_back(&mut vec, i);
            i = i + 1;
        };

        let sum: u64 = 0;

        while (count > 0) {
            let val = std::bcs::to_bytes(&vec);
            sum = sum + ((*std::vector::borrow(&val, 0)) as u64);
            count = count - 1;
        }
    }
}
