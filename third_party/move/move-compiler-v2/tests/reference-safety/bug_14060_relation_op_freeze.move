module 0xCAFE::Module0 {
    public fun f() {
        let x = &mut 0u8;
        let y = &mut 1u8;
        x != copy y;
    }
}
