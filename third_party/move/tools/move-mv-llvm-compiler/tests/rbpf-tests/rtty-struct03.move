// log 0000000000000000000000000000000000000000000000000000000000000200::ST::A1 {254, 36893488147419103232, 0, }
// log [0000000000000000000000000000000000000000000000000000000000000200::ST::A1 {254, 36893488147419103232, 0, }, 0000000000000000000000000000000000000000000000000000000000000200::ST::A1 {123, 456, 0, }, ]
// log 123
// log 456


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
    struct A1 has drop, copy {
        f1: u8,
        f2: u128
    }

    public fun new(a1: u8, a2: u128): A1 {
        A1 { f1: a1, f2: a2 }
    }

    public fun get(s: &A1): (u8, u128) {
        let A1 { f1: x, f2: y } = *s;
        (x, y)
    }
}

script {
  use 0x10::debug;
  use 0x10::vector;
  use 0x200::ST;

  fun main() {
    let s1 = ST::new(254, 36893488147419103232_u128);
    let s2 = ST::new(123, 456);
    debug::print(&s1);

    // Now we're really going gonzo and operating on and debug-printing
    // a vector-of-structs.
    let v: vector<ST::A1> = vector::empty();
    vector::push_back(&mut v, s1);
    vector::push_back(&mut v, s2);
    debug::print(&v);

    let sref = vector::borrow<ST::A1>(&v, 1);
    let (f1, f2) = ST::get(sref);
    debug::print(&f1);
    debug::print(&f2);
  }
}
