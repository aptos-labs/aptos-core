module 0x42::M {
    use std::option;
    use std::vector;

    spec S {
        invariant x > 20;
    }

    struct IsolatedPosition has key {
        position: S
    }

    struct CrossedPosition has key {
        positions: vector<S>
    }

    struct S has store {
        x: u64
    }

    struct Foo has key {
        x: u64
    }

    public(package) fun bar(x: u64): option::Option<Foo> {
        if (x > 20) {
            option::some(Foo { x: x })
        } else {
            option::none()
        }
    }

    inline fun must_find_position_mut(
        address: address, x: u64
    ): &mut S {
        if (exists<IsolatedPosition>(address)) {
            &mut borrow_global_mut<IsolatedPosition>(address).position
        } else {
            let positions = &mut CrossedPosition[address].positions;
            let (is_found, index) = vector::find(positions, |position| &position.x == &x);
            if (is_found) {
                vector::borrow_mut(positions, index)
            } else {
                abort 1
            }
        }
    }

    inline fun must_find_position(
        address: address, x: u64
    ): &S {
        if (exists<IsolatedPosition>(address)) {
            &borrow_global<IsolatedPosition>(address).position
        } else {
            let positions = &mut CrossedPosition[address].positions;
            let (is_found, index) = vector::find(positions, |position| &position.x == &x);
            if (is_found) {
                vector::borrow(positions, index)
            } else {
                abort 1
            }
        }
    }

    fun get_s_error(
        address: address, x: u64
    ): option::Option<Foo> acquires IsolatedPosition, CrossedPosition {
        let s = must_find_position_mut(address, x);
        bar(s.x)
    }

    fun get_s_no_error(
        address: address, x: u64
    ): option::Option<Foo> acquires IsolatedPosition, CrossedPosition {
        let s = must_find_position(address, x);
        bar(s.x)
    }

    fun test_input_param_as_mut_ref(r: &mut S) {
        let y = &mut r.x;
        spec {
            assert r.x == y;
        };
    }
}
