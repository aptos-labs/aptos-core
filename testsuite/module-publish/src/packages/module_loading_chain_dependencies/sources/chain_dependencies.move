module publisher_address::chain_dependencies {

    public entry fun run() {
        let sum = publisher_address::chain_dependencies_1::next();
        assert!(sum == 20, 77);
    }
}

module publisher_address::chain_dependencies_1 {

    public fun next(): u64 {
        1 + publisher_address::chain_dependencies_2::next()
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

module publisher_address::chain_dependencies_2 {

    public fun next(): u64 {
        1 + publisher_address::chain_dependencies_3::next()
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

module publisher_address::chain_dependencies_3 {

    public fun next(): u64 {
        1 + publisher_address::chain_dependencies_4::next()
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

module publisher_address::chain_dependencies_4 {

    public fun next(): u64 {
        1 + publisher_address::chain_dependencies_5::next()
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

module publisher_address::chain_dependencies_5 {

    public fun next(): u64 {
        1 + publisher_address::chain_dependencies_6::next()
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

module publisher_address::chain_dependencies_6 {

    public fun next(): u64 {
        1 + publisher_address::chain_dependencies_7::next()
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

module publisher_address::chain_dependencies_7 {

    public fun next(): u64 {
        1 + publisher_address::chain_dependencies_8::next()
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

module publisher_address::chain_dependencies_8 {

    public fun next(): u64 {
        1 + publisher_address::chain_dependencies_9::next()
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

module publisher_address::chain_dependencies_9 {

    public fun next(): u64 {
        1 + publisher_address::chain_dependencies_10::next()
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

module publisher_address::chain_dependencies_10 {

    public fun next(): u64 {
        1 + publisher_address::chain_dependencies_11::next()
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

module publisher_address::chain_dependencies_11 {

    public fun next(): u64 {
        1 + publisher_address::chain_dependencies_12::next()
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

module publisher_address::chain_dependencies_12 {

    public fun next(): u64 {
        1 + publisher_address::chain_dependencies_13::next()
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

module publisher_address::chain_dependencies_13 {

    public fun next(): u64 {
        1 + publisher_address::chain_dependencies_14::next()
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

module publisher_address::chain_dependencies_14 {

    public fun next(): u64 {
        1 + publisher_address::chain_dependencies_15::next()
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

module publisher_address::chain_dependencies_15 {

    public fun next(): u64 {
        1 + publisher_address::chain_dependencies_16::next()
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

module publisher_address::chain_dependencies_16 {

    public fun next(): u64 {
        1 + publisher_address::chain_dependencies_17::next()
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

module publisher_address::chain_dependencies_17 {

    public fun next(): u64 {
        1 + publisher_address::chain_dependencies_18::next()
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

module publisher_address::chain_dependencies_18 {

    public fun next(): u64 {
        1 + publisher_address::chain_dependencies_19::next()
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

module publisher_address::chain_dependencies_19 {

    public fun next(): u64 {
        1 + publisher_address::chain_dependencies_20::next()
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

module publisher_address::chain_dependencies_20 {
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
