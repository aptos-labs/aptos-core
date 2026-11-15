// An opaque call's `aborts_of<callee>` / `!aborts_of<callee>` aggregates
// describe that call, so they are stated over the memory at the call. A
// caller's `aborts_if aborts_of<callee>(..)` reads the predicate at the
// caller's entry; the two agree exactly when nothing the callee reads was
// written in between, which is what makes the clause true or false.
//
// Positives: one call with nothing written before it; two calls with the
// state unchanged between them. Negatives, which must fail: a write to the
// callee's state before the call, with the clause stated at entry; and the
// same with two calls on the same argument, where the first call's success
// must not make the second call's abort unreachable.
module 0x42::behavioral_call_state {
    struct Light has key {
        on: bool
    }

    fun open_door() acquires Light {
        assert!(!borrow_global<Light>(@0x42).on, 1);
    }
    spec open_door {
        pragma opaque;
        aborts_if !exists<Light>(@0x42);
        aborts_if global<Light>(@0x42).on;
    }

    fun open_once() acquires Light {
        open_door();
    }
    spec open_once {
        aborts_if aborts_of<open_door>();
    }

    fun open_twice_same_state() acquires Light {
        open_door();
        open_door();
    }
    spec open_twice_same_state {
        aborts_if aborts_of<open_door>();
    }

    // The light is switched on first, so the door aborts whatever the entry
    // state said: must fail.
    fun switch_on_then_open() acquires Light {
        borrow_global_mut<Light>(@0x42).on = true;
        open_door();
    }
    spec switch_on_then_open {
        aborts_if !exists<Light>(@0x42);
        aborts_if exists<Light>(@0x42) && aborts_of<open_door>();
    }

    // The first call succeeds with the light off; the second aborts after the
    // switch. The clause at entry says no abort: must fail.
    fun open_switch_open() acquires Light {
        open_door();
        borrow_global_mut<Light>(@0x42).on = true;
        open_door();
    }
    spec open_switch_open {
        aborts_if !exists<Light>(@0x42);
        aborts_if exists<Light>(@0x42) && aborts_of<open_door>();
    }

    // A callee over two resources, of which the caller writes only one before
    // the call. Both are snapshotted at the call; the unwritten one must not
    // pull the other back to the entry state. Must fail.
    struct Fuse has key {
        blown: bool
    }

    fun open_guarded() acquires Light, Fuse {
        assert!(!borrow_global<Light>(@0x42).on, 1);
        assert!(!borrow_global<Fuse>(@0x42).blown, 2);
    }
    spec open_guarded {
        pragma opaque;
        aborts_if !exists<Light>(@0x42);
        aborts_if !exists<Fuse>(@0x42);
        aborts_if global<Light>(@0x42).on;
        aborts_if global<Fuse>(@0x42).blown;
    }

    fun blow_then_open() acquires Light, Fuse {
        borrow_global_mut<Fuse>(@0x42).blown = true;
        open_guarded();
    }
    spec blow_then_open {
        aborts_if !exists<Light>(@0x42);
        aborts_if !exists<Fuse>(@0x42);
        aborts_if exists<Light>(@0x42) && exists<Fuse>(@0x42) && aborts_of<open_guarded>();
    }

    // The honest clause for the same body verifies.
    fun open_switch_open_honest() acquires Light {
        open_door();
        borrow_global_mut<Light>(@0x42).on = true;
        open_door();
    }
    spec open_switch_open_honest {
        aborts_if true;
    }
}
