//# publish
module 0x2::a {
    // Empty module for 0x2::b to link against when declaring as a friend.
}

//# publish
module 0x2::b {
    friend 0x2::a;

    fun private_function<T>() {}
    public(friend) fun friend_function<T>() {}
    public fun public_function<T>() {}
}


//# publish
module 0x2::a {
    use 0x2::b;

    public fun call_private_function() {
        let f = || b::private_function<u8>();
        f();
    }
    public fun call_friend_function() {
        let f = || b::friend_function<u8>();
        f();
    }

    public fun call_public_function() {
        let f = || b::public_function<u8>();
        f();
    }
}

//# publish
module 0x2::c {
    use 0x2::b;

    public fun call_private_function() {
        let f = || b::private_function<u8>();
        f();
    }
    public fun call_friend_function() {
        let f = || b::friend_function<u8>();
        f();
    }

    public fun call_public_function() {
        let f = || b::public_function<u8>();
        f();
    }
}

//# run 0x2::a::call_private_function

//# run 0x2::a::call_friend_function

//# run 0x2::a::call_public_function

//# run 0x2::c::call_private_function

//# run 0x2::c::call_friend_function

//# run 0x2::c::call_public_function

//# run --signers 0x1
script {
    use 0x2::b;

    fun main(_account: signer) {
        let f = || b::private_function<u8>();
        f();
    }
}

//# run --signers 0x1
script {
    use 0x2::b;

    fun main(_account: signer) {
        let f = || b::friend_function<u8>();
        f();
    }
}

//# run --signers 0x1
script {
    use 0x2::b;

    fun main(_account: signer) {
        let f = || b::public_function<u8>();
        f();
    }
}
