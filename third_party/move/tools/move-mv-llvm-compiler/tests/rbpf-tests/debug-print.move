// log 10
// log 11
// log 12
// log 13
// log 14
// log 15
// log @00000000000000000000000000000000000000000000000000000000000000AB
// log true

// log [1]
// log [2]
// log [3]
// log [4]
// log [5]
// log [6]


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

script {
  use 0x10::debug;
  use 0x10::vector;

  fun main() {
    debug::print(&10_u8);
    debug::print(&11_u16);
    debug::print(&12_u32);
    debug::print(&13_u64);
    debug::print(&14_u128);
    debug::print(&15_u256);
    debug::print(&@0xAB);
    debug::print(&true);

    let v: vector<u8> = vector::empty();
    vector::push_back(&mut v, 1);
    debug::print(&v);

    let v: vector<u16> = vector::empty();
    vector::push_back(&mut v, 2);
    debug::print(&v);

    let v: vector<u32> = vector::empty();
    vector::push_back(&mut v, 3);
    debug::print(&v);

    let v: vector<u64> = vector::empty();
    vector::push_back(&mut v, 4);
    debug::print(&v);

    let v: vector<u128> = vector::empty();
    vector::push_back(&mut v, 5);
    debug::print(&v);

    let v: vector<u256> = vector::empty();
    vector::push_back(&mut v, 6);
    debug::print(&v);
  }
}
