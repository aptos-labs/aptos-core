module 0x123::a{
    public(friend) entry inline fun a(){}
}

module 0x123::b{
    entry inline fun a(){}

    fun b() {
        a()
    }
}
