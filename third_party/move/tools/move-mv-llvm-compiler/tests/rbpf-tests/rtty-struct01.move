// log 0000000000000000000000000000000000000000000000000000000000000200::ST::A1 {123000, 66000, 33000, @000000000000000000000000000000000000000000000000000000000000CAFE, 0, }
// log 123000
// log 66000
// log 33000
// log @000000000000000000000000000000000000000000000000000000000000CAFE

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
        f1: u64,
        f2: u64,
        f3: u16,
        f4: address
    }

    public fun new(a1: u64, a2: u64, a3: u16, a4: address): A1 {
        A1 { f1: a1, f2: a2, f3: a3, f4: a4 }
    }

    public fun get(s: &A1): (u64, u64, u16, address) {
        let A1 { f1: x, f2: y, f3: z, f4: w} = *s;
        (x, y, z, w)
    }
}

script {
  use 0x10::debug;
  use 0x10::vector;
  use 0x200::ST;

  fun main() {
    let s = ST::new(123000, 66000, 33000, @0xcafe);
    // We can debug print a structure now!
    debug::print(&s);

    let v: vector<ST::A1> = vector::empty();
    vector::push_back(&mut v, s);
    let sref = vector::borrow<ST::A1>(&v, 0);
    let (f1, f2, f3, f4) = ST::get(sref);
    debug::print(&f1);
    debug::print(&f2);
    debug::print(&f3);
    debug::print(&f4);
  }
}
