module 0xCAFE::Module0 {
    struct S {
        x: bool,
    }
    public fun f(): S {
        let s = S { x: false, };
        *&mut {s.x} = true;
        s
    }
}
