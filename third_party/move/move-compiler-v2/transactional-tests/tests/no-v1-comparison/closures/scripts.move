//# publish
module 0xc0ffee::m {
    public fun foo() {}
}

//# run
script {
    fun main() {
        use 0xc0ffee::m;
        let f = m::foo;
        f();
    }
}

//# run
script {
    fun main() {
        let f = || 0xc0ffee::m::foo();
        f();
    }
}

//# run
script {
    fun main() {
        (0xc0ffee::m::foo)();
    }
}
