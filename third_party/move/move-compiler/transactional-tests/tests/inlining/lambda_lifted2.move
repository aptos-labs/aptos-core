//# publish
module 0x42::example2 {
    inline fun twice(f: |u64|u64, x: u64): u64 {
        f(f(x))
    }

    entry fun foo(): u64 {
        let a = 2;
        twice(|z| { a = a + 1; z * a }, 3)
    }

    entry fun foo1(): u64 {
        let a = 2;
        {
            let z = {
                let z = 3;
                a = a + 1;
                z * a
            };
            a = a + 1;
            z * a
        }
    }

    fun lifted_lambda(ap: &mut u64, z: u64): u64 {
        *ap = *ap + 1;
        z * *ap
    }

    entry fun foo2(): u64 {
        let a = 2;
        let z = 3;
        let r = lifted_lambda(&mut a, z);
        lifted_lambda(&mut a, r)
    }
}

//# run 0x42::example2::foo

//# run 0x42::example2::foo1

//# run 0x42::example2::foo2
