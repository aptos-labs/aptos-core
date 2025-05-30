//# publish
module 0xAA00::M0 {
    struct S1 has drop;
}

//# publish
module 0xAA00::M1 {
    use 0xAA00::M0;
    public entry fun f1() {
        (
            | _x: |M0::S1 |has drop | { }
        )(
            | _x: M0::S1 | { }
        );
    }

}
