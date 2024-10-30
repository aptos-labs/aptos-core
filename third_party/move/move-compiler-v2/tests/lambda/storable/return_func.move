module 0x42::test {
    fun double(x: u64): u64 {
        x * 2
    }

    fun triple(x: u64) : u64 {
        x * 3
    }

    public fun multiply(x: u64, y: u64): u64 {
        x * y
    }

    public fun multiply3(x: u64, y: u64, z: u64): u64 {
        x * y * z
    }

    fun multiply_by_x(x: u64): |u64|u64: store {
        multiply(x, _)
    }

    fun choose_function(key: u64) : |u64|u64: store {
        if (key == 0) {
            &double
        } else if (key == 1) {
            &triple
        } else {
            multiply_by_x(4)
        }
    }

    fun choose_function2(key: u64): |u64|u64: store {
        if (key == 0) {
            move |x| double(x);
        } else if (key == 1) {
            move |x| triple(x);
        } else if (key == 2) {
            let f = multiply_by_x(4);
            move |x| f(x)
        } else if (key == 3) {
            let f = multiply_by_x(5);
            move |x| f(x)
        } else if (key == 4) {
            let f = multiply(6, _);
            f
        } else if (key == 5) {
            multiply(_, 7)
        } else if (key == 6) {
            let f = multiply(6, _);
            move |x| f(x)
        } else if (key == 7) {
            let f = multiply_by_x(5);
            move |x| f(x)
        } else {
            multiply3(_, 3, 4)
        }
    }

    fun choose_function3(key: u64) : |u64|u64: store {
        if (key == 0) {
            let f = move |x| double(x);
            f
        } else if (key == 1) {
            let g = move |x| triple(x);
            g
        } else {
            let h = multiply_by_x(4);
            h
        }
    }

    public fun test_functions(choose_function: |u64|(|u64|u64: store)) {
        let sum = vector[];
        let x = 3;
        sum.push_back(choose_function(0)(x));
        sum.push_back(choose_function(1)(x));
        sum.push_back(choose_function(2)(x));

        let g = choose_function(1)(x);
        let h = choose_function(2)(x);
        let f = choose_function(0)(x);

        sum.push_back(f(5));
        sum.push_back(g(7));
        sum.push_back(h(6));

        let funcs = vector[choose_function(0), choose_function(1), choose_function(2)];
        sum.push_back(f[0](9));
        sum.push_back(f[1](11));
        sum.push_back(f[2](13));

        assert!(vector<u64>[6, 9, 12, 10, 21, 24, 18, 33, 52])
    }

    public fun test_function_choosers() {
        test_functions(&choose_function);
        test_functions(&choose_function2);
        test_functions(&choose_function3);
    }
}
