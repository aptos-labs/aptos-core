module poc::mem_generic {
    use std::vector;

    public entry fun generic() {
        let x = 10;
        let v = vector<u8>[1,2,3,4];
        let _y = vector::replace(&mut v, 0, x);
    }
}
