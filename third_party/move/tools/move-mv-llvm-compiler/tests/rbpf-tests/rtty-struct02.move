// log 0000000000000000000000000000000000000000000000000000000000000200::ST::A1 {456, 0000000000000000000000000000000000000000000000000000000000000200::ST::A0 {true, 0, }, 0, }
// log 456
// log true


module 0x10::debug {
  native public fun print<T>(x: &T);
}

module 0x10::vector {
  native public fun empty<Element>(): vector<Element>;
  native public fun length<Element>(v: &vector<Element>): u64;
  native public fun push_back<Element>(v: &mut vector<Element>, e: Element);
  native public fun pop_back<Element>(v: &mut vector<Element>): Element;
  native public fun destroy_empty<Element>(v: vector<Element>);
  native public fun swap<Element>(v: &mut vector<Element>, i: u64, j: u64);
  native public fun borrow<Element>(v: &vector<Element>, i: u64): &Element;
  native public fun borrow_mut<Element>(v: &mut vector<Element>, i: u64): &mut Element;
}

module 0x200::ST {
    struct A0 has drop, copy {
        a: bool
    }

    struct A1 has drop, copy {
        f1: u64,
        f2: A0
    }

    public fun new(a1: u64, inner_a: bool): A1 {
        let sa0 = A0 { a: inner_a };
        A1 { f1: a1, f2: sa0 }
    }

    public fun get(s: &A1): (u64, bool) {
        let A1 { f1: x, f2: y } = *s;
        (x, y.a)
    }
}

script {
  use 0x10::debug;
  use 0x10::vector;
  use 0x200::ST;

  fun main() {
    let s = ST::new(456, true);
    // We can print a nested struct too.
    debug::print(&s);

    let v: vector<ST::A1> = vector::empty();
    vector::push_back(&mut v, s);

    let sref = vector::borrow<ST::A1>(&v, 0);
    let (f1, f2) = ST::get(sref);
    debug::print(&f1);
    debug::print(&f2);
  }
}
