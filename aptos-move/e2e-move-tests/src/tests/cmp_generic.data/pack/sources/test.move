 /// Module for testing non-integer primitive types
 module 0x99::primitive_cmp {
    //* bool group
    fun test_left_lt_right_bool(x: bool, y: bool): bool {
        // a and b are created to test our optimization
        let a = &x;
        let b = &y;

        *a < *b &&
        x < y
    }
    fun test_left_le_right_bool(x: bool, y: bool): bool {
        x <= y
    }
    fun test_left_gt_right_bool(x: bool, y: bool): bool {
        x > y
    }
    fun test_left_ge_right_bool(x: bool, y: bool): bool {
        x >= y
    }

    //* address group
    fun test_left_lt_right_address(x: address, y: address): bool {
        // a and b are created to test our optimization
        let a = &x;
        let b = &y;

        *a < *b &&
        x < y
    }
    fun test_left_le_right_address(x: address, y: address): bool {
        x <= y
    }
    fun test_left_gt_right_address(x: address, y: address): bool {
        x > y
    }
    fun test_left_ge_right_address(x: address, y: address): bool {
        x >= y
    }

    //* vector group
    fun test_left_lt_right_vector(x: vector<u8>, y: vector<u8>): bool {
        // a and b are created to test our optimization
        let a = &x;
        let b = &y;

        *a < *b &&
        x < y
    }
    fun test_left_le_right_vector(x: vector<u8>, y: vector<u8>): bool {
        x <= y
    }
    fun test_left_gt_right_vector(x: vector<u8>, y: vector<u8>): bool {
        x > y
    }
    fun test_left_ge_right_vector(x: vector<u8>, y: vector<u8>): bool {
        x >= y
    }

    //* nested vector group
    fun test_left_lt_right_nested_vector(x: vector<vector<u8>>, y: vector<vector<u8>>): bool {
        // a and b are created to test our optimization
        let a = &x;
        let b = &y;

        *a < *b &&
        x < y
    }
    fun test_left_le_right_nested_vector(x: vector<vector<u8>>, y: vector<vector<u8>>): bool {
        x <= y
    }
    fun test_left_gt_right_nested_vector(x: vector<vector<u8>>, y: vector<vector<u8>>): bool {
        x > y
    }
    fun test_left_ge_right_nested_vector(x: vector<vector<u8>>, y: vector<vector<u8>>): bool {
        x >= y
    }

    //* entry functions for testing non-integer primitive types
    entry fun test_bool(){
        let x = false;
        let y = true;
        assert!(test_left_lt_right_bool(x, y), 0);
        assert!(test_left_le_right_bool(x, y), 0);
        assert!(test_left_le_right_bool(x, x), 0);
        assert!(test_left_le_right_bool(y, y), 0);

        assert!(test_left_gt_right_bool(y, x), 0);
        assert!(test_left_ge_right_bool(y, x), 0);
        assert!(test_left_ge_right_bool(x, x), 0);
        assert!(test_left_ge_right_bool(y, y), 0);
    }

    entry fun test_address(){
        let x = @0x1;
        let y = @0x2;
        assert!(test_left_lt_right_address(x, y), 0);
        assert!(test_left_le_right_address(x, y), 0);
        assert!(test_left_le_right_address(x, x), 0);
        assert!(test_left_le_right_address(y, y), 0);

        assert!(test_left_gt_right_address(y, x), 0);
        assert!(test_left_ge_right_address(y, x), 0);
        assert!(test_left_ge_right_address(x, x), 0);
        assert!(test_left_ge_right_address(y, y), 0);
    }

    entry fun test_vector(){
        let x = vector[0u8, 1u8, 2u8, 3u8, 4u8, 5u8];
        let y = vector[0u8, 1u8, 2u8, 3u8, 4u8, 5u8, 6u8];
        let z = vector[1u8, 2u8, 3u8, 4u8, 5u8, 6u8];

        assert!(test_left_lt_right_vector(x, y), 0);
        assert!(test_left_lt_right_vector(y, z), 0);
        assert!(test_left_lt_right_vector(x, z), 0);

        assert!(test_left_le_right_vector(x, y), 0);
        assert!(test_left_le_right_vector(y, z), 0);
        assert!(test_left_le_right_vector(x, z), 0);
        assert!(test_left_le_right_vector(x, x), 0);
        assert!(test_left_le_right_vector(y, y), 0);
        assert!(test_left_le_right_vector(z, z), 0);

        assert!(test_left_gt_right_vector(y, x), 0);
        assert!(test_left_gt_right_vector(z, y), 0);
        assert!(test_left_gt_right_vector(z, x), 0);

        assert!(test_left_ge_right_vector(y, x), 0);
        assert!(test_left_ge_right_vector(z, y), 0);
        assert!(test_left_ge_right_vector(z, x), 0);
        assert!(test_left_ge_right_vector(x, x), 0);
        assert!(test_left_ge_right_vector(y, y), 0);
        assert!(test_left_ge_right_vector(z, z), 0);
    }

    entry fun test_nested_vector(){
        let x = vector[0u8, 1u8, 2u8, 3u8, 4u8, 5u8];
        let y = vector[0u8, 1u8, 2u8, 3u8, 4u8, 5u8, 6u8];
        let z = vector[1u8, 2u8, 3u8, 4u8, 5u8, 6u8];

        let nested_1 = vector[x];
        let nested_2 = vector[x, x];
        let nested_3 = vector[x, y];
        let nested_4 = vector[x, y, z];
        let nested_5 = vector[x, z];
        let nested_6 = vector[y];
        let nested_7 = vector[y, y];
        let nested_8 = vector[y, z];
        let nested_9 = vector[z];
        let nested_10 = vector[z, z];

        assert!(test_left_lt_right_nested_vector(nested_1, nested_2), 0);
        assert!(test_left_lt_right_nested_vector(nested_2, nested_3), 0);
        assert!(test_left_lt_right_nested_vector(nested_3, nested_4), 0);
        assert!(test_left_lt_right_nested_vector(nested_4, nested_5), 0);
        assert!(test_left_lt_right_nested_vector(nested_5, nested_6), 0);
        assert!(test_left_lt_right_nested_vector(nested_6, nested_7), 0);
        assert!(test_left_lt_right_nested_vector(nested_7, nested_8), 0);
        assert!(test_left_lt_right_nested_vector(nested_8, nested_9), 0);
        assert!(test_left_lt_right_nested_vector(nested_9, nested_10), 0);

        assert!(test_left_le_right_nested_vector(nested_1, nested_2), 0);
        assert!(test_left_le_right_nested_vector(nested_2, nested_3), 0);
        assert!(test_left_le_right_nested_vector(nested_3, nested_4), 0);
        assert!(test_left_le_right_nested_vector(nested_4, nested_5), 0);
        assert!(test_left_le_right_nested_vector(nested_5, nested_6), 0);
        assert!(test_left_le_right_nested_vector(nested_6, nested_7), 0);
        assert!(test_left_le_right_nested_vector(nested_7, nested_8), 0);
        assert!(test_left_le_right_nested_vector(nested_8, nested_9), 0);
        assert!(test_left_le_right_nested_vector(nested_9, nested_10), 0);

        assert!(test_left_gt_right_nested_vector(nested_2, nested_1), 0);
        assert!(test_left_gt_right_nested_vector(nested_3, nested_2), 0);
        assert!(test_left_gt_right_nested_vector(nested_4, nested_3), 0);
        assert!(test_left_gt_right_nested_vector(nested_5, nested_4), 0);
        assert!(test_left_gt_right_nested_vector(nested_6, nested_5), 0);
        assert!(test_left_gt_right_nested_vector(nested_7, nested_6), 0);
        assert!(test_left_gt_right_nested_vector(nested_8, nested_7), 0);
        assert!(test_left_gt_right_nested_vector(nested_9, nested_8), 0);
        assert!(test_left_gt_right_nested_vector(nested_10, nested_9), 0);

        assert!(test_left_ge_right_nested_vector(nested_2, nested_1), 0);
        assert!(test_left_ge_right_nested_vector(nested_3, nested_2), 0);
        assert!(test_left_ge_right_nested_vector(nested_4, nested_3), 0);
        assert!(test_left_ge_right_nested_vector(nested_5, nested_4), 0);
        assert!(test_left_ge_right_nested_vector(nested_6, nested_5), 0);
        assert!(test_left_ge_right_nested_vector(nested_7, nested_6), 0);
        assert!(test_left_ge_right_nested_vector(nested_8, nested_7), 0);
        assert!(test_left_ge_right_nested_vector(nested_9, nested_8), 0);
        assert!(test_left_ge_right_nested_vector(nested_10, nested_9), 0);
    }
}

/// Module for struct types
 module 0x99::struct_cmp {
    use std::cmp;
    /// A simple struct
    struct Int has drop, copy {
        a: u8,
        b: u16,
        c: u32,
        d: u64,
        e: u128,
        f: u256
    }

    /// A more complex struct
    struct Complex has drop, copy {
        a: u8,
        b: Int,
    }

    /// A more complex struct with vectors
    struct ComplexWithVec has drop, copy {
        a: u8,
        b: Int,
        c: vector<Int>,
        d: vector<vector<Complex>>
    }

    //* Simple struct group
    fun test_simple_struct_lt(x: Int, y: Int): bool {
        // a and b are created to test our optimization
        let a = &x;
        let b = &y;

        *a < *b &&
        x < y
    }
    fun test_simple_struct_le(x: Int, y: Int): bool {
        x <= y
    }
    fun test_simple_struct_gt(x: Int, y: Int): bool {
        x > y
    }
    fun test_simple_struct_ge(x: Int, y: Int): bool {
        x >= y
    }

    //* Complex struct group
    fun test_complex_struct_lt(x: Complex, y: Complex): bool {
        // a and b are created to test our optimization
        let a = &x;
        let b = &y;

        *a < *b &&
        x < y
    }
    fun test_complex_struct_le(x: Complex, y: Complex): bool {
        x <= y
    }
    fun test_complex_struct_gt(x: Complex, y: Complex): bool {
        x > y
    }
    fun test_complex_struct_ge(x: Complex, y: Complex): bool {
        x >= y
    }

    //* Complex struct with vector group
    fun test_complex_struct_vec_lt(x: ComplexWithVec, y: ComplexWithVec): bool {
        // a and b are created to test our optimization
        let a = &x;
        let b = &y;

        *a < *b &&
        x < y &&
        x.b < y.b &&
        x.c < y.c &&
        x.c[0] < y.c[0] &&
        x.d < y.d &&
        x.d[0] < y.d[0] &&
        x.d[0][0] < y.d[0][0]
    }

    //* Enum as special struct group
    fun test_special_struct_vec_lt(x: ComplexWithVec, y: ComplexWithVec): bool {
        // a and b are created to test our optimization
        let a = &cmp::compare(&x, &y);
        let b = &cmp::compare(&y, &x);

        *a < *b &&
        cmp::compare(&x, &y) < cmp::compare(&y, &x) &&
        cmp::compare(&x.b, &y.b) < cmp::compare(&y.b, &x.b) &&
        cmp::compare(&x.c, &y.c) < cmp::compare(&y.c, &x.c) &&
        cmp::compare(&x.c[0], &y.c[0]) < cmp::compare(&y.c[0], &x.c[0]) &&
        cmp::compare(&x.d, &y.d) < cmp::compare(&y.d, &x.d) &&
        cmp::compare(&x.d[0], &y.d[0]) < cmp::compare(&y.d[0], &x.d[0]) &&
        cmp::compare(&x.d[0][0], &y.d[0][0]) < cmp::compare(&y.d[0][0], &x.d[0][0])
    }

     //* entry functions for testing struct types
    entry fun test_simple_struct(){
        let x = Int {
            a: 1,
            b: 2,
            c: 3,
            d: 4,
            e: 5,
            f: 6
        };
        let y = Int {
            a: 2,
            b: 3,
            c: 4,
            d: 5,
            e: 6,
            f: 7
        };

        assert!(test_simple_struct_lt(x, y), 0);
        assert!(test_simple_struct_le(x, y), 0);
        assert!(test_simple_struct_le(x, x), 0);
        assert!(test_simple_struct_le(y, y), 0);

        assert!(test_simple_struct_gt(y, x), 0);
        assert!(test_simple_struct_ge(y, x), 0);
        assert!(test_simple_struct_ge(x, x), 0);
        assert!(test_simple_struct_ge(y, y), 0);
    }

    entry fun test_complex_struct(){

        let x = Complex {
            a: 1,
            b: Int {
                a: 1,
                b: 2,
                c: 3,
                d: 4,
                e: 5,
                f: 6
            }
        };

        let y = Complex {
            a: 2,
            b: Int {
                a: 1,
                b: 2,
                c: 3,
                d: 4,
                e: 5,
                f: 6
            }
        };

        assert!(test_complex_struct_lt(x, y), 0);
        assert!(test_complex_struct_le(x, y), 0);
        assert!(test_complex_struct_le(x, x), 0);
        assert!(test_complex_struct_le(y, y), 0);

        assert!(test_complex_struct_gt(y, x), 0);
        assert!(test_complex_struct_ge(y, x), 0);
        assert!(test_complex_struct_ge(x, x), 0);
        assert!(test_complex_struct_ge(y, y), 0);
    }

     entry fun test_nested_complex_struct(){
        let x = ComplexWithVec {
            a: 1,
            b: Int {
                a: 1,
                b: 2,
                c: 3,
                d: 4,
                e: 5,
                f: 6
            },
            c: vector[
                Int {
                    a: 1,
                    b: 2,
                    c: 3,
                    d: 4,
                    e: 5,
                    f: 6
                },
            ],
            d: vector[
                vector[
                    Complex {
                        a: 1,
                        b: Int {
                            a: 1,
                            b: 2,
                            c: 3,
                            d: 4,
                            e: 5,
                            f: 6
                        }
                    }
                ]
            ],
        };

        let y = ComplexWithVec {
            a: 2,
            b: Int {
                a: 2,
                b: 2,
                c: 3,
                d: 4,
                e: 5,
                f: 6
            },
            c: vector[
                Int {
                    a: 2,
                    b: 2,
                    c: 3,
                    d: 4,
                    e: 5,
                    f: 6
                },
            ],
            d: vector[
                vector[
                    Complex {
                        a: 2,
                        b: Int {
                            a: 2,
                            b: 2,
                            c: 3,
                            d: 4,
                            e: 5,
                            f: 6
                        }
                    }
                ]
            ],
        };

        assert!(test_complex_struct_vec_lt(x, y), 0);
     }

     entry fun test_special_complex_struct(){
        let x = ComplexWithVec {
            a: 1,
            b: Int {
                a: 1,
                b: 2,
                c: 3,
                d: 4,
                e: 5,
                f: 6
            },
            c: vector[
                Int {
                    a: 1,
                    b: 2,
                    c: 3,
                    d: 4,
                    e: 5,
                    f: 6
                },
            ],
            d: vector[
                vector[
                    Complex {
                        a: 1,
                        b: Int {
                            a: 1,
                            b: 2,
                            c: 3,
                            d: 4,
                            e: 5,
                            f: 6
                        }
                    }
                ]
            ],
        };

        let y = ComplexWithVec {
            a: 2,
            b: Int {
                a: 2,
                b: 2,
                c: 3,
                d: 4,
                e: 5,
                f: 6
            },
            c: vector[
                Int {
                    a: 2,
                    b: 2,
                    c: 3,
                    d: 4,
                    e: 5,
                    f: 6
                },
            ],
            d: vector[
                vector[
                    Complex {
                        a: 2,
                        b: Int {
                            a: 2,
                            b: 2,
                            c: 3,
                            d: 4,
                            e: 5,
                            f: 6
                        }
                    }
                ]
            ],
        };

        assert!(test_special_struct_vec_lt(x, y), 0);
     }
}

/// Module for testing generic types
 module 0x99::generic_cmp {
    use std::cmp;
    public struct Int has drop, copy {
        a: u8,
        b: u16,
        c: u32,
        d: u64,
        e: u128,
        f: u256
    }

    public struct Complex has drop, copy {
        a: u8,
        b: Int,
    }

    struct Foo<T> has drop, copy { x: T }

    struct Bar<T1, T2> has drop, copy {
        x: T1,
        y: vector<T2>,
    }

    //* Simple generic arg group
    fun test_generic_arg_lt<T: drop + copy>(x: T, y: T): bool {
        // a and b are created to test our optimization
        let a = &cmp::compare(&x, &y);
        let b = &cmp::compare(&y, &x);

        *a < *b &&
        x < y
    }
    fun test_generic_arg_le<T: drop + copy>(x: T, y: T): bool {
        x <= y
    }
    fun test_generic_arg_gt<T: drop + copy>(x: T, y: T): bool {
        x > y
    }
    fun test_generic_arg_ge<T: drop + copy>(x: T, y: T): bool {
        x >= y
    }


    //* Simple generic struct arg group
    fun test_generic_struct_lt(x: Foo<address>, y: Foo<address>): bool {
        // a and b are created to test our optimization
        let a = &cmp::compare(&x, &y);
        let b = &cmp::compare(&y, &x);

        *a < *b &&
        x < y &&
        x.x < y.x
    }
    fun test_generic_struct_le(x: Foo<address>, y: Foo<address>): bool {

        x <= y &&
        x.x <= y.x
    }
    fun test_generic_struct_gt(x: Foo<address>, y: Foo<address>): bool {
        x > y &&
        x.x > y.x
    }
    fun test_generic_struct_ge(x: Foo<address>, y: Foo<address>): bool {
        x >= y &&
        x.x >= y.x
    }

    //* Complex generic struct arg group
    public fun test_generic_complex_struct_lt(x: Bar<Int, Complex>, y: Bar<Int, Complex>): bool {
         // a and b are created to test our optimization
        let a = &cmp::compare(&x, &y);
        let b = &cmp::compare(&y, &x);

        *a < *b &&
        x < y &&
        x.x < y.x &&
        x.y < y.y &&
        x.y[0] < y.y[0]
    }
    public fun test_generic_complex_struct_le(x: Bar<Int, Complex>, y: Bar<Int, Complex>): bool {
        x <= y &&
        x.x <= y.x &&
        x.y <= y.y &&
        x.y[0] <= y.y[0]
    }
    public fun test_generic_complex_struct_gt(x: Bar<Int, Complex>, y: Bar<Int, Complex>): bool {
        x > y &&
        x.x > y.x &&
        x.y > y.y &&
        x.y[0] > y.y[0]
    }
    public fun test_generic_complex_struct_ge(x: Bar<Int, Complex>, y: Bar<Int, Complex>): bool {
        x >= y &&
        x.x >= y.x &&
        x.y >= y.y &&
        x.y[0] >= y.y[0]
    }

    //* entry functions for testing generic types
    entry fun test_generic_arg(){
        let x = @0x1;
        let y = @0x2;
        assert!(test_generic_arg_lt(x, y), 0);
        assert!(test_generic_arg_le(x, y), 0);
        assert!(test_generic_arg_le(x, x), 0);
        assert!(test_generic_arg_le(y, y), 0);

        assert!(test_generic_arg_gt(y, x), 0);
        assert!(test_generic_arg_ge(y, x), 0);
        assert!(test_generic_arg_ge(x, x), 0);
        assert!(test_generic_arg_ge(y, y), 0);

    }

    entry fun test_generic_struct(){
        let x = Foo<address> {x: @0x1};
        let y = Foo<address> {x: @0x2};

        assert!(test_generic_struct_lt(x, y), 0);
        assert!(test_generic_struct_le(x, y), 0);
        assert!(test_generic_struct_le(x, x), 0);
        assert!(test_generic_struct_le(y, y), 0);

        assert!(test_generic_struct_gt(y, x), 0);
        assert!(test_generic_struct_ge(y, x), 0);
        assert!(test_generic_struct_ge(x, x), 0);
        assert!(test_generic_struct_ge(y, y), 0);
    }

     entry fun test_generic_complex_struct(){
        let x = Bar<Int, Complex> {
            x: Int{
                a: 1,
                b: 2,
                c: 3,
                d: 4,
                e: 5,
                f: 6
            },
            y: vector[
                Complex{
                    a: 1,
                    b: Int {
                        a: 1,
                        b: 2,
                        c: 3,
                        d: 4,
                        e: 5,
                        f: 6
                    }
                }
            ]
        };

        let y = Bar<Int, Complex> {
            x: Int{
                a: 2,
                b: 2,
                c: 3,
                d: 4,
                e: 5,
                f: 6
            },
            y: vector[
                Complex{
                    a: 2,
                    b: Int {
                        a: 2,
                        b: 2,
                        c: 3,
                        d: 4,
                        e: 5,
                        f: 6
                    }
                }
            ]
        };

        assert!(test_generic_complex_struct_lt(x, y), 0);
        assert!(test_generic_complex_struct_le(x, y), 0);
        assert!(test_generic_complex_struct_le(x, x), 0);
        assert!(test_generic_complex_struct_le(y, y), 0);

        assert!(test_generic_complex_struct_gt(y, x), 0);
        assert!(test_generic_complex_struct_ge(y, x), 0);
        assert!(test_generic_complex_struct_ge(x, x), 0);
        assert!(test_generic_complex_struct_ge(y, y), 0);
     }
}

/// Modules for testing function values
module 0x99::module1 {
    public fun test(): u64{
        1
    }
    public fun test1(): u64{
        1
    }
    public fun test2<T: drop>(_x: T){
    }
    public fun test3(x: u64): u64{
        x + 1
    }
}
module 0x99::module2 {
    public fun test(): u64{
        1
    }
}

/// Function values are compared in the following order:
/// 1. Module identification is compared by address and name
/// 2. Function name is compared based on identity string
/// 3. Type parameters are compared based on types (by discriminant index in their defining enum)
/// 4. Captured values are compared

module 0x99::function_value_cmp {
    use 0x99::module1 as module1;
    use 0x99::module2 as module2;

    //* entry function for testing function values
    entry fun test_module_name_cmp(){
        // f1 < f2 due to module name `module1` < `module2`
        // - `f1` named to `closure#0module1::test;`
        // - `f2` named to `closure#0module2::test;`
        let f1: ||u64 has drop = module1::test;
        let f2: ||u64 has drop = module2::test;
        assert!(f1 < f2, 0);
    }
    entry fun test_function_name_cmp(){
        // f1 < f2 due to function name `test` < `test1`
        // - `f1` named to `closure#0module1::test;`
        // - `f2` named to `closure#0module1::test1;`
        let f1: ||u64 has drop = module1::test;
        let f2: ||u64 has drop = module1::test1;
        assert!(f1 < f2, 0);

        // f3 < f4 due to function name by lambda order
        // - `f3` named to `closure#0function_value_cmp::__lambda__1__test_function_name_cmp;`
        // - `f4` named to `closure#0function_value_cmp::__lambda__2__test_function_name_cmp;`
        let f3: ||u64 has drop = ||1;
        let f4: ||u64 has drop = ||1;
        assert!(f3 < f4, 0);

        // f5 < f6 due to function name by lambda order
        // - `f5` named to `closure#0function_value_cmp::__lambda__3__test_function_name_cmp;`
        // - `f6` named to `closure#0function_value_cmp::__lambda__4__test_function_name_cmp;`
        let f5: ||u64 has drop = ||1;
        let f6: ||u64 has drop = ||100;
        assert!(f5 < f6, 0);
    }
    entry fun test_typed_arg_cmp(){
        // f1 < f2 due to type parameter `u8` < `u64`
        let x: u8 = 1;
        let y: u64 = 1;
        let f1: || has drop = ||module1::test2(x);
        let f2: || has drop = ||module1::test2(y);
        assert!(f1 < f2, 0);
    }
    entry fun test_captured_var_cmp(){
        // f1 < f2 due to captured values `1` < `2`
        let x = 1;
        let y = 2;
        let f1: ||u64 has drop = ||module1::test3(x);
        let f2: ||u64 has drop = ||module1::test3(y);
        assert!(f1 < f2, 0);
    }
}
