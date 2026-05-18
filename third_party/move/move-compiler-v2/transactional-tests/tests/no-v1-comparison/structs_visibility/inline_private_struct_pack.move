//# publish
module 0xA::A {
    struct W(|| has drop) has drop;
    public fun f() {}
    public inline fun make_w(): W {
        f
    }
}

//# publish
module 0xB::B {
    use 0xA::A;

    fun test(): 0xA::A::W {
        A::make_w()
    }
}
