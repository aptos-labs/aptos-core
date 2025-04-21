module 0xc0ffee::m {
    use std::string::{Self, String};
    use std::vector;

    fun test1_warn(keys: &vector<String>) {
        vector::for_each_ref(
            keys,
            |k| {
                let pre = string::sub_string(&*k, 0, 5);
                assert!(pre != string::utf8(b"hello"), 0);
            }
        )
    }

    fun test1_no_warn(keys: &vector<String>) {
        vector::for_each_ref(
            keys,
            |k| {
                let pre = string::sub_string(k, 0, 5);
                assert!(pre != string::utf8(b"hello"), 0);
            }
        )
    }

    fun test2_no_warn(x: &u64) {
        let y = &mut *x;
        *y = 4;
    }

    fun test_3_no_warn(x: &u64) {
        *&mut *x = 4;
    }

    fun test_4_warn(x: &u64): &u64 {
        &*x
    }

    #[lint::skip(needless_ref_deref)]
    fun test_5_no_warn(x: &u64): &u64 {
        &*x
    }
}
