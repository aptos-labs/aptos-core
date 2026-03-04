// Publish consumer first as a stub so provider can declare it as a friend.
//# publish
module 0x42::consumer {}

//# publish
module 0x42::provider {
    friend 0x42::consumer;
    friend const FRIEND_VALUE: u64 = 55;
}

//# publish
module 0x42::consumer {
    use 0x42::provider;

    public fun get(): u64 {
        provider::FRIEND_VALUE
    }
}

//# run
script {
    use 0x42::consumer;
    fun main() {
        assert!(consumer::get() == 55, 1);
    }
}
