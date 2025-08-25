module 0xc0ffee::m {
    public fun test1_warn(){
        {
            let x1 = 1;
            x1 += 1;
            x1
        };
    }

    public fun test2_warn(){
        {
            let x2 = 2;
            impure(&mut x2);
            x2
        };
    }

    public fun test3_warn(x: bool){
        if (x){
            let x2 = 3;
            impure(&mut x2);
            x2
        }else{
            let x1 = 4;
            x1 += 1;
            x1
        };
    }

    public fun test4_warn(): u64{
        let x4 = 1;
        {
            pure({
                let x5 = 5;
                x5
            });
            x4
        }
    }

    public fun test5_warn(n: u64): u64{
        {
            let x = 6;
            let n2 = n;
            loop{
                if (n == 0){
                    break;
                };
                x += 1;
                n2 -= 1;
            };
        };
        0
    }

    public fun test6_warn(n: u64){
        pure({
            let x = 6;
            let n2 = n;
            loop{
                if (n == 0){
                    break;
                };
                x += 1;
                n2 -= 1;
            };
            x
        });
    }

    public fun test7_warn(): u64{
        let x3 = 8;
        {
            x3 += 1;
            x3
        };
        x3
    }

    /*****************************************************/

    public fun test1_no_warn(): u64{
        let x3 = 7;
        {
            x3 += 1;
        };
        x3
    }

    public fun test2_no_warn(): u64{
        let x3 = 8;
        {
            x3 += 1;
            x3
        }
    }

    struct S has key, drop {
        x: u64,
    }

    public fun test3_no_warn(addr: address) acquires S{
        {
            let x1 = 9;
            borrow_global_mut<S>(addr).x += 1;
            x1 += 1;
            x1
        };
    }

    public fun test4_no_warn(addr: address) acquires S{
        {
            let x1 = 10;
            impure2(addr);
            x1 += 1;
            x1
        };
    }

    public fun test5_no_warn(x: bool): u64{
        let x3 = 11;
        if (x){
            let x2 = 12;
            impure(&mut x2);
            x2
        }else{
            x3 += 1;
            x3
        };
        x3
    }

    public fun test6_no_warn(x: bool): u64{
        let x3 = 11;
        if (x){
            let x2 = 12;
            impure(&mut x2);
            x3 += 1;
            x2
        }else{
            x3
        };
        x3
    }

    public fun test7_no_warn(x: bool): u64{
        let x3 = 11;
        if (x){
            let x2 = 12;
            impure(&mut x2);
            x3 += 1;
            x2
        }else{
            x3 += 1;
            x3
        };
        x3
    }

    public fun test8_no_warn(depth: u64){
        //Note: recursive_a(), recursive_b(), and recursive_c() are actually
        //pure, but recursive functions are too difficult to analyze. An ideal
        //linter would say that this call can be removed. This test is not for
        //correctness, it's just to make sure we don't hang or crash.
        recursive_a(depth, 42);
        recursive_b(depth, 42);
        recursive_c(depth, 42);
    }

    public fun test9_no_warn(addr: address) acquires S{
        impure3(addr, {
            let x2 = 2;
            impure(&mut x2);
            x2
        });
    }

    public fun test10_no_warn(): u64{
        let x4 = 1;
        {
            pure({
                let x5 = 5;
                x4 += 1;
                x5
            });
            x4
        }
    }

    public fun test11_no_warn(): u64{
        let x4 = 1;
        x4 += 1;
        pure({
            let x5 = 5;
            x4 += 1;
            x5 += 1;
            x5
        });
        x4
    }

    /*****************************************************/

    fun pure(x7: u64): u64{
        x7
    }

    fun impure(x8: &mut u64){
        *x8 += 1;
    }

    fun impure2(addr: address): bool acquires S{
        borrow_global_mut<S>(addr).x += 1;
        true
    }

    fun impure3(addr: address, x: u64): bool acquires S{
        borrow_global_mut<S>(addr).x += x;
        true
    }

    fun recursive_a(depth: u64, x: u64): u64{
        if (depth == 0){
            return x;
        };
        recursive_b(depth - 1, x + 1)
    }

    fun recursive_b(depth: u64, x: u64): u64{
        if (depth == 0){
            return x;
        };
        recursive_a(depth - 1, x + 1)
    }

    fun recursive_c(depth: u64, x: u64): u64{
        if (depth == 0){
            return x;
        };
        recursive_c(depth - 1, x + 1)
    }

}
