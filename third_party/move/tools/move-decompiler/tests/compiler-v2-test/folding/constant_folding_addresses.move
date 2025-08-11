module 0x8675309::M {
    fun const_addr(): address {
        @0x1234
    }
    fun const_addr_let(): address {
        let x = @0x1234;
        x
    }
}
