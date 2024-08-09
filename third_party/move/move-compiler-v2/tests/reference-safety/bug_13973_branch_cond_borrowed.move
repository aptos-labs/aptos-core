module 0xCAFE::Module0 {
    public fun f1(x: bool) {
        let y: &mut bool =  &mut x;
        if (x) {
        } else {
            *y= false;
        };
        if (copy x) { } else { };
    }
}
