//# init --addresses alice=0x42

//# publish
module alice::pattern {
    public fun value(): u64 {
        42
    }
}

//# run --signers alice
script {
    fun main() {
        assert!(alice::pattern::value() == 42);
    }
}
