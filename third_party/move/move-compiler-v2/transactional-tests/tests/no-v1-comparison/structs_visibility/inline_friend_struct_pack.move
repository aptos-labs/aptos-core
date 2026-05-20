//# publish
module 0xA::A {
    friend struct W(|| has drop) has drop;
    public fun f() {}
    public inline fun make_w(): W {
        f
    }
}

//# publish
module 0xA::B {
    use 0xA::A;

    fun test(): A::W {
        A::make_w()
    }
}
