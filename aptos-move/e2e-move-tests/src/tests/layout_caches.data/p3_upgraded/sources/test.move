module 0xcafe::m3 {

    enum M3 has key, drop {
        V0 {
            x: u64,
            m2: 0xcafe::m2::M2,
        },
        V1 {
            dummy: 0xcafe::d1::D1,
        }
    }

    fun init_module(acc: &signer) {
        move_to(acc, M3::V0 { x: 0, m2: 0xcafe::m2::m2() })
    }

    public entry fun load_m3() {
        let m3 = borrow_global<M3>(@0xcafe);
        assert!(m3.x == 0, 777);
    }

    public entry fun load_m3_with_extra_module() {
        0xcafe::dummy::dummy();
        load_m3();
    }
}

module 0xcafe::dummy {
    public fun dummy() {}
}
