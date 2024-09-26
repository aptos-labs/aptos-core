module 0x8675309::M {
    use std::vector;

    public inline fun pass_mut_ref<T>(v: &mut vector<T>, action: |&mut T|&mut T): &mut T {
        action(vector::borrow_mut(v, 0))
    }

    public inline fun pass_mut2_ref<T>(v: &mut vector<T>, action: |&mut T|&mut T): &T {
        action(vector::borrow_mut(v, 0))
    }

    public inline fun pass_mut3_ref<T>(v: &mut vector<T>, action: |&mut T|&T): &T {
        action(vector::borrow_mut(v, 0))
    }

    public inline fun pass_mut4_ref<T>(v: &mut vector<T>, action: |&T|&T): &T {
        action(vector::borrow_mut(v, 0))
    }

    public inline fun pass_imm_ref<T>(v: &mut vector<T>, action: |&T|&T): &T {
        return action(vector::borrow(v, 0))
    }

    // 1

    public fun consume_mut_mut() {
        let v = vector[1, 2, 3];
        let r = pass_mut_ref(&mut v, |x: &mut u64| x);
    }

    public fun consume_mut_imm() {
        let v = vector[1, 2, 3];
        let r = pass_mut_ref(&mut v, |x: &u64| x);
    }

    public fun consume_mut_untyped() {
        let v = vector[1, 2, 3];
        let r = pass_mut_ref(&mut v, |x| x);
    }

    public fun consume_mut_freeze() {
        let v = vector[1, 2, 3];
        let r = pass_mut_ref(&mut v, |x: &mut u64| (freeze(x)));
    }

    // 2

    public fun consume_mut2_mut() {
        let v = vector[1, 2, 3];
        let r = pass_mut2_ref(&mut v, |x: &mut u64| x);
    }

    public fun consume_mut2_imm() {
        let v = vector[1, 2, 3];
        let r = pass_mut2_ref(&mut v, |x: &u64| x);
    }

    public fun consume_mut2_untyped() {
        let v = vector[1, 2, 3];
        let r = pass_mut2_ref(&mut v, |x| x);
    }

    public fun consume_mut2_freeze() {
        let v = vector[1, 2, 3];
        let r = pass_mut2_ref(&mut v, |x: &mut u64| (freeze(x)));
    }

    // 3

    public fun consume_mut3_mut() {
        let v = vector[1, 2, 3];
        let r = pass_mut3_ref(&mut v, |x: &mut u64| x);
    }

    public fun consume_mut3_imm() {
        let v = vector[1, 2, 3];
        let r = pass_mut3_ref(&mut v, |x: &u64| x);
    }

    public fun consume_mut3_untyped() {
        let v = vector[1, 2, 3];
        let r = pass_mut3_ref(&mut v, |x| x);
    }

    public fun consume_mut3_freeze() {
        let v = vector[1, 2, 3];
        let r = pass_mut3_ref(&mut v, |x: &mut u64| (freeze(x)));
    }

    // 4

    public fun consume_mut4_mut() {
        let v = vector[1, 2, 3];
        let r = pass_mut4_ref(&mut v, |x: &mut u64| x);
    }

    public fun consume_mut4_imm() {
        let v = vector[1, 2, 3];
        let r = pass_mut4_ref(&mut v, |x: &u64| x);
    }

    public fun consume_mut4_untyped() {
        let v = vector[1, 2, 3];
        let r = pass_mut4_ref(&mut v, |x| x);
    }

    public fun consume_mut4_freeze() {
        let v = vector[1, 2, 3];
        let r = pass_mut4_ref(&mut v, |x: &mut u64| (freeze(x)));
    }

    // imm

    public fun consume_imm_mut() {
        let v = vector[1, 2, 3];
        let r = pass_imm_ref(&mut v, |x: &mut u64| x);
    }

    public fun consume_imm_imm() {
        let v = vector[1, 2, 3];
        let r = pass_imm_ref(&mut v, |x: &u64| x);
    }

    public fun consume_imm_untyped() {
        let v = vector[1, 2, 3];
        let r = pass_imm_ref(&mut v, |x| x);
    }

    public fun consume_imm_freeze() {
        let v = vector[1, 2, 3];
        let r = pass_imm_ref(&mut v, |x: &mut u64| (freeze(x)));
    }
}
