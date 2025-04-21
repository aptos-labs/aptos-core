module 0xc0ffee::m {
    use std::vector;

    fun id(x: u64): u64 {
        x
    }

    fun bytes(_x: &u64): vector<u8> {
        vector[1u8, 2u8]
    }

    fun cons_2(x: u64, _y: &mut u64): u64 {
        x
    }

    fun one(): u64 {
        1
    }

    fun cons_7(_x: vector<u8>, _a: u64, _b: u64, _c: u64, _d: u64, _e: u64, _f: u64): u64 {
        0
    }

    fun cons_2_another(_x: &u64, _y: u64) {}

    fun test(new_address: u64): u64 {
        let new_account = id(new_address);
        let authentication_key = bytes(&new_address);
        assert!(
            vector::length(&authentication_key) == 2,
            42
        );

        let guid_creation_num = 0;

        let guid_for_coin = cons_2(new_address, &mut guid_creation_num);
        let coin_register_events = id(guid_for_coin);

        let guid_for_rotation = cons_2(new_address, &mut guid_creation_num);
        let key_rotation_events = id(guid_for_rotation);

        cons_2_another(
            &new_account,
            cons_7(
                authentication_key,
                0,
                guid_creation_num,
                coin_register_events,
                key_rotation_events,
                id(one()),
                id(one()),
            )
        );

        new_account
    }
}
