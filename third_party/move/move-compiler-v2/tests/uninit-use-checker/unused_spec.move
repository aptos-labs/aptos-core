module 0xCAFE::Module0 {
    spec module {
         global y: num;

    }
    public fun test_1() {
        let z;
        spec {
            assert z == 3;
        };
        z = 2;
    }

    public fun test_2() {
        let z;
        spec {
            update y = z;
        };
        z = 2;
    }
}
