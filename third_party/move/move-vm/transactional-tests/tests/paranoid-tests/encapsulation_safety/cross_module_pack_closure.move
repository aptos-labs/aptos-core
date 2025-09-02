//# publish
module 0x22::a {
    // Empty module for 0x22::b to link against when declaring as a friend.
}

//# publish
module 0x22::b {
    friend 0x22::a;

    fun private_function() {}
    public(friend) fun friend_function() {}
    public fun public_function() {}
}


//# publish
module 0x22::a {
    use 0x22::b;

    public fun call_private_function() {
        let f = || b::private_function();
        f();
    }
    public fun call_friend_function() {
        let f = || b::friend_function();
        f();
    }

    public fun call_public_function() {
        let f = || b::public_function();
        f();
    }
}

//# publish
module 0x22::c {
    use 0x22::b;

    public fun call_private_function() {
        let f = || b::private_function();
        f();
    }
    public fun call_friend_function() {
        let f = || b::friend_function();
        f();
    }

    public fun call_public_function() {
        let f = || b::public_function();
        f();
    }
}

//# run 0x22::a::call_private_function

//# run 0x22::a::call_friend_function

//# run 0x22::a::call_public_function

//# run 0x22::c::call_private_function

//# run 0x22::c::call_friend_function

//# run 0x22::c::call_public_function

//# run --signers 0x1
script {
    use 0x22::b;

    fun main(_account: signer) {
        let f = || b::private_function();
        f();
    }
}

//# run --signers 0x1
script {
    use 0x22::b;

    fun main(_account: signer) {
        let f = || b::friend_function();
        f();
    }
}

//# run --signers 0x1
script {
    use 0x22::b;

    fun main(_account: signer) {
        let f = || b::public_function();
        f();
    }
}
