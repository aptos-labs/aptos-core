//# publish
module 0x42::consts {
    public const NEG_ONE: i64 = -1;
    public const MIN_I8: i8 = -128;
}

//# publish
module 0x42::consumer {
    use 0x42::consts;

    public fun check_neg_one(): bool {
        consts::NEG_ONE == -1i64
    }

    public fun check_min_i8(): bool {
        consts::MIN_I8 == -128i8
    }
}

//# run
script {
    use 0x42::consumer;
    fun main() {
        assert!(consumer::check_neg_one(), 1);
        assert!(consumer::check_min_i8(), 2);
    }
}
