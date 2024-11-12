module publisher_address::chain_friends {
    friend publisher_address::chain_friends_1;

    public entry fun run() {
        // no-op
    }
}

module publisher_address::chain_friends_1 {
    friend publisher_address::chain_friends_2;

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

module publisher_address::chain_friends_2 {
    friend publisher_address::chain_friends_3;

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

module publisher_address::chain_friends_3 {
    friend publisher_address::chain_friends_4;

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

module publisher_address::chain_friends_4 {
    friend publisher_address::chain_friends_5;

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

module publisher_address::chain_friends_5 {
    friend publisher_address::chain_friends_6;

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

module publisher_address::chain_friends_6 {
    friend publisher_address::chain_friends_7;

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

module publisher_address::chain_friends_7 {
    friend publisher_address::chain_friends_8;

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

module publisher_address::chain_friends_8 {
    friend publisher_address::chain_friends_9;

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

module publisher_address::chain_friends_9 {
    friend publisher_address::chain_friends_10;

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

module publisher_address::chain_friends_10 {
    friend publisher_address::chain_friends_11;

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

module publisher_address::chain_friends_11 {
    friend publisher_address::chain_friends_12;

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

module publisher_address::chain_friends_12 {
    friend publisher_address::chain_friends_13;

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

module publisher_address::chain_friends_13 {
    friend publisher_address::chain_friends_14;

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

module publisher_address::chain_friends_14 {
    friend publisher_address::chain_friends_15;

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

module publisher_address::chain_friends_15 {
    friend publisher_address::chain_friends_16;

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

module publisher_address::chain_friends_16 {
    friend publisher_address::chain_friends_17;

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

module publisher_address::chain_friends_17 {
    friend publisher_address::chain_friends_18;

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

module publisher_address::chain_friends_18 {
    friend publisher_address::chain_friends_19;

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

module publisher_address::chain_friends_19 {
    friend publisher_address::chain_friends_20;

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

module publisher_address::chain_friends_20 {
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
