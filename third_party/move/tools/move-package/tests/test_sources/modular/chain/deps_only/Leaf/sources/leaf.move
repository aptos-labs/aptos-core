module leaf::leaf {
    public fun add(x: u64, y: u64): u64 { x + y }
    fun private_impl(): u64 { 1 }
}
