module 0x42::inline_specs {

    fun succ(x: u64): u64 {
        x + 1
    }

    fun specs(): u64 {
        let x = 0;
        spec { assert x == 0;  };
        x = succ(x);
        spec { assert x == 1;  };
        x
    }
}
