module 0xCAFE::Module0 {
    public fun f1(x: bool) {
        let y: &mut bool =  &mut x;
        if (x) {
        } else {
            *y= false;
        };
        if (copy x) { } else { };
    }

    public fun f2(x: bool) {
        let y: &mut bool =  &mut x;
        while (x) {
            *y= false;
        };
        if (copy x) { } else { };
    }

}
