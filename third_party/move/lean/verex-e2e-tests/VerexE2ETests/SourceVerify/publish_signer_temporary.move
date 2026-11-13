/// The temporary borrow of the signer `move_to` takes ends with it, so the
/// signer can move on afterwards.
module 0x42::publish_signer_temporary {
    struct R has key { v: u64 }

    fun publish(s: signer): signer {
        move_to(&s, R { v: 1 });
        s
    }
    spec publish {
        aborts_if exists<R>(std::signer::address_of(s));
        ensures global<R>(std::signer::address_of(s)).v == 1;
        ensures result == s;
    }
}
