module 0xcafe::test_add {
    //use aptos_std::debug;

    public entry fun calibrate_add_1() {
        let i = 0;
        let _ = i + 1;
    }

    public entry fun calibrate_add_2() {
        let i = 0;
        let _ = i + i + 1;
    }

    /*public entry fun calibrate_add_2() {
        let i = 0;
        let a = 5;
        let b = 11;
        while (i < 1000) {
            let _ = a + b + b;
            i = i + 1;
        }
    }

    public entry fun calibrate_add_3() {
        let i = 0;
        while (i < 1000) {
            let _ = 42;
            i = i + 1;
        }
    }

    public entry fun calibrate_add_4() {
        let i = 0;
        let a = 1;
        let b = a + a;
        while (i < 1000) {
            let _ = 42 + b;
            i = i + 1;
        }
    }

    public entry fun calibrate_add_5() {
        let i = 0;
        let a = 1;
        let b = a + a;
        while (i < 1000 && i < 1000) {
            let _ = 42 + b;
            i = i + 1;
        }
    }*/

    /*public entry fun calibrate_add_3() {
        let a = 5;
        let b = 12;
        let _ = a + b;
    }

    public entry fun calibrate_add_4() {
        let a = 5;
        let b = 13;
        let _ = a + b;
    }

    public entry fun calibrate_add_5() {
        let a = 5;
        let b = 14;
        let _ = a + b;
    }

    public entry fun calibrate_add_6() {
        let a = 5;
        let b = 15;
        let _ = a + b;
    }

    public entry fun calibrate_add_7() {
        let a = 5;
        let b = 15;
        let _ = a + b;
    }

    public entry fun calibrate_add_8() {
        let a = 5;
        let b = 15;
        let _ = a + b;
    }

    public entry fun calibrate_add_9() {
        let a = 5;
        let b = 15;
        let _ = a + b;
    }*/
}
