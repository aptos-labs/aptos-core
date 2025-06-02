//# publish --print-bytecode
module 0x42::mod2 {
    struct Registry<F: store+copy> has key, store {
        func: F
    }

    public fun save_item<F: store+copy>(owner: &signer, f: F) {
        move_to<Registry<F>>(owner, Registry { func: f });
    }

    public fun item_exists<F: store+copy>(addr: address): bool {
        exists<Registry<F>>(addr)
    }

    public fun get_item<F: store+copy>(addr: address): F acquires Registry {
        borrow_global<Registry<F>>(addr).func
    }
}

//# publish --print-bytecode
module 0x42::mod3 {
    use std::signer;

    struct MyStruct1 has key, store, copy {
        x: u64
    }

    struct MyStruct2 has key, store, copy {
        y: u8
    }

    public fun test_items(owner: signer, use_1: bool) {
        let struct1 = MyStruct1 { x: 3 };
        // let struct2 = MyStruct2 { y: 2 };

        let f1 : |address|bool has drop+store+copy = |addr| 0x42::mod2::item_exists<MyStruct1>(addr);
        let f2 : |address|bool has drop+store+copy = |addr| 0x42::mod2::item_exists<MyStruct2>(addr);

        let addr = signer::address_of(&owner);
        0x42::mod2::save_item(&owner, struct1);

        // Store just MyStruct1
        move_to<MyStruct1>(&owner, struct1);

        // Store f1 or f2, depending on use_1
        if (use_1) {
            0x42::mod2::save_item(&owner, f1);
        } else {
            0x42::mod2::save_item(&owner, f2);
        };

        // In either case, item exists
        assert!(0x42::mod2::item_exists<|address|bool has store+copy>(addr));

        let found_f = 0x42::mod2::get_item<|address|bool has store+copy>(addr);

        assert!(use_1 == found_f(addr));
    }

    public fun test_item1(owner: signer) {
        test_items(owner, true);
    }

    public fun test_item2(owner: signer) {
        test_items(owner, false);
    }
}

//# run --signers 0x42 -- 0x42::mod3::test_item1

//# run --signers 0x42 -- 0x42::mod3::test_item2
