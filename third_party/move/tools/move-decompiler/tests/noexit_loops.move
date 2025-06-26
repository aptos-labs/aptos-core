//* Test cases with no-exit loops
module 0x99::noexit_loops {
    fun f1() {
        let x = 0;
        'outer: loop {
            // inner loop; can never go out
            loop {
                x = x + 1;
                'inner: loop if (true) loop {
                    if (false) continue 'outer else break 'inner;
                    break
                } else continue 'outer
            };
            break
        }
    }

    fun f2(): u64 {
        let x = 1;
        loop { x = x + 1; break };
        loop { x = x + 1 };
        x
    }

    fun foo(x: u64): u64 {
        x = x + 1;
        x
    }
    fun f3(): u64 {
        let x = 1;
        while (true) {
            x = x + foo(x)
        };
        x
    }

    struct R {}

    fun f4(): R {
        loop {}
    }

    fun f5(): u64 {
        loop { let x = 0; x; }
    }

    fun bar(_x: u64) {}
    fun f6() {
        bar(loop {})
    }

    fun f7(): R {
        let x: R = loop { 0; };
        x
    }

    fun f8() {
        let () = loop { break };
        let () = loop { if (false) break };
    }

    fun f9() {
        while (true) ();
        while (false) ()
    }

    fun f10() {
        while ({ let foo = true; foo }) ();
        while ({ let bar = false; bar }) ()
    }

    fun f11() {
        loop {
            continue;
            break
        };
    }

    fun f12() {
        loop {
            if (return) break;
        }
    }

    fun baz(_: &u64) {}
    fun f13(cond: bool) {
        1 + if (cond) 0 else { 1 } + 2;
        1 + loop {} + 2;
        1 + return + 0;

        baz(&if (cond) 0 else 1);
        baz(&loop {});
        baz(&return);
        baz(&abort 0);
    }

    fun f14(p: bool, q: bool) {
        while (p) {
            if (q) {
                loop {};
                let i = 0;
                i = i + 1;
            } else {
                break;
            };
            let i = 0;
            i = i + 1;
        }
    }

    fun f15(){
         let x = 0;
        'outer: loop {
            // inner loop; just run once
            'inner: loop {
                x = x + 1;
                'innermost: loop {
                    if (true) loop {
                        if (false) continue 'outer else break 'inner;
                        break
                    } else continue 'outer
                }
            };
            break
        }

    }

    fun f16() {
        loop { loop { loop {}; }; };
    }
}
