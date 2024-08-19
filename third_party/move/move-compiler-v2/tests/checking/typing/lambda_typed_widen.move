module 0x8675309::M {
    use std::vector;

    public inline fun use_mut_ref<T>(v: &mut vector<T>, action: |&mut T|T): T {
        action(vector::borrow_mut(v, 0))
    }

    public inline fun use_imm_ref<T>(v: &mut vector<T>, action: |&T|T): T {
        return action(vector::borrow(v, 0))
    }

    public fun consume_mut_mut() {
        let v = vector[1, 2, 3];
        let r = use_mut_ref(&mut v, |x: &mut u64| *x);
    }

    public fun consume_mut_imm() {
        let v = vector[1, 2, 3];
        let r = use_mut_ref(&mut v, |x: &u64| *x);
    }

    public fun consume_mut_untyped() {
        let v = vector[1, 2, 3];
        let r = use_mut_ref(&mut v, |x| *x);
    }

    public fun consume_mut_untyped2() {
        let v = vector[1, 2, 3];
        let r = use_mut_ref(&mut v, |x| *(freeze(x)));
    }

    public fun consume_imm_mut() {
        let v = vector[1, 2, 3];
        let r = use_imm_ref(&mut v, |x: &mut u64| *x);
    }

    public fun consume_imm_imm() {
        let v = vector[1, 2, 3];
        let r = use_imm_ref(&mut v, |x: &u64| *x);
    }

    public fun consume_imm_untyped() {
        let v = vector[1, 2, 3];
        let r = use_imm_ref(&mut v, |x| *x);
    }

    public fun consume_imm_untyped2() {
        let v = vector[1, 2, 3];
        let r = use_imm_ref(&mut v, |x| *(freeze(x)));
    }
}
