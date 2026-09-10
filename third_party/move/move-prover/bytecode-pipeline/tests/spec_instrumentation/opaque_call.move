// Contains tests for treatment of opaque calls
module 0x42::Test {

	struct R has key { v: u64 }

	fun get_and_incr(addr: address): u64 acquires R {
	    if (!exists<R>(addr)) abort 33;
	    let r = borrow_global_mut<R>(addr);
	    let v = r.v;
	    r.v = r.v + 1;
	    v
	}
	spec get_and_incr {
	    pragma opaque;
	    requires addr != @0x0;
	    aborts_if !exists<R>(addr) with 33;
	    aborts_if global<R>(addr).v + 1 >= 18446744073709551615;
	    modifies global<R>(addr);
	    ensures global<R>(addr).v == old(global<R>(addr)).v + 1;
	    ensures result == global<R>(addr).v;
	}

	fun incr_twice() acquires R {
	    get_and_incr(@0x1);
	    get_and_incr(@0x1);
	}
	spec incr_twice {
	    aborts_if !exists<R>(@0x1) with 33;
	    ensures global<R>(@0x1).v == old(global<R>(@0x1)).v + 2;
	}

    fun set_without_frame(addr: address, value: u64) acquires R {
        borrow_global_mut<R>(addr).v = value;
    }
    spec set_without_frame {
        pragma opaque;
        requires exists<R>(addr);
        ensures global<R>(addr).v == value;
    }

    fun call_without_frame(addr: address, value: u64) acquires R {
        set_without_frame(addr, value);
    }
    spec call_without_frame {
        requires exists<R>(addr);
        ensures global<R>(addr).v == value;
    }

    struct Generic<T> has key { v: T }

    fun set_generic<T: drop + store>(addr: address, other: address, value: T) acquires Generic {
        borrow_global_mut<Generic<T>>(addr).v = value;
        borrow_global_mut<Generic<u8>>(other).v = 0;
    }
    spec set_generic {
        pragma opaque;
        modifies global<Generic<T>>(addr);
    }

    fun call_generic_frame(addr: address, other: address) acquires Generic {
        set_generic<bool>(addr, other, true);
    }

    fun call_generic_collision(addr: address, other: address) acquires Generic {
        set_generic<u8>(addr, other, 1);
    }
}
