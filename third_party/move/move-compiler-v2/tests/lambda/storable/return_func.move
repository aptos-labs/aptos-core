module 0x42::test {
    use std::vector;

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

    fun multiply_by_x(x: u64): |u64|u64 with store {
        move |y| multiply(x, y) with store
    }

    fun choose_function(key: u64) : |u64|u64 with store {
        if (key == 0) {
            double
        } else if (key == 1) {
            triple
        } else {
            multiply_by_x(4)
        }
    }

    fun choose_function2(key: u64): |u64|u64 with store {
        if (key == 0) {
            move |x| double(x) with store
        } else if (key == 1) {
            move |x| triple(x) with store
        } else if (key == 2) {
            let f = multiply_by_x(4);
            move |x| f(x) with store
        } else if (key == 3) {
            let f = multiply_by_x(5);
            move |x| f(x) with store
        } else if (key == 4) {
            let f = move |y| multiply(6, y) with store;
            f
        } else if (key == 5) {
            move |y| multiply(y, 7) with store
        } else if (key == 6) {
            let f = move |y| multiply(6, y) with store;
            move |x| f(x) with store
        } else if (key == 7) {
            let f = multiply_by_x(5);
            move |x| f(x) with store
        } else {
            move |y| multiply3(y, 3, 4) with store
        }
    }

    fun choose_function3(key: u64) : |u64|u64 with store {
        if (key == 0) {
            let f = move |x| double(x) with store;
            f
        } else if (key == 1) {
            let g = move |x| triple(x) with store;
            g
        } else {
            let h = multiply_by_x(4);
            h
        }
    }

    public fun test_functions(choose_function: |u64|(|u64|u64 with store)) {
        let sum = vector<u64>[];
        let x = 3;
        vector::push_back(&mut sum, (choose_function(0))(x));
        vector::push_back(&mut sum, (choose_function(1))(x));
        vector::push_back(&mut sum, (choose_function(2))(x));

        let g = choose_function(1)(x);
        let h = choose_function(2)(x);
        let f = choose_function(0)(x);

        vector::push_back(&mut sum, f(5));
        vector::push_back(&mut sum, g(7));
        vector::push_back(&mut sum, h(6));

        let funcs = vector[choose_function(0), choose_function(1), choose_function(2)];
        vector::push_back(&mut sum, (funcs[0])(9));
        vector::push_back(&mut sum, (funcs[1])(11));
        vector::push_back(&mut sum, (funcs[2])(13));

        assert!(sum == vector<u64>[6, 9, 12, 10, 21, 24, 18, 33, 52])
    }

    public fun test_function_choosers() {
        test_functions(choose_function);
        test_functions(choose_function2);
        test_functions(choose_function3);
    }
}
