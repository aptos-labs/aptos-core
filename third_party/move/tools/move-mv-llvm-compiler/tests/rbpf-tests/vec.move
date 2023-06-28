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

module 0x10::tests {
  use 0x10::vector;


  public fun test_u8() {
    let v: vector<u8> = vector::empty();

    let len = vector::length(&v);
    assert!(len == 0, 10);

    vector::push_back(&mut v, 2);
    vector::push_back(&mut v, 3);

    let len = vector::length(&v);
    assert!(len == 2, 10);

    vector::swap(&mut v, 0, 1);

    let elt = vector::borrow(&v, 0);
    assert!(*elt == 3, 10);
    let elt = vector::borrow_mut(&mut v, 0);
    assert!(*elt == 3, 10);

    let elt = vector::pop_back(&mut v);
    assert!(elt == 2, 10);
    let elt = vector::pop_back(&mut v);
    assert!(elt == 3, 10);

    vector::destroy_empty(v);
  }

  public fun test_u16() {
    let v: vector<u16> = vector::empty();

    let len = vector::length(&v);
    assert!(len == 0, 10);

    vector::push_back(&mut v, 2);
    vector::push_back(&mut v, 3);

    let len = vector::length(&v);
    assert!(len == 2, 10);

    vector::swap(&mut v, 0, 1);

    let elt = vector::borrow(&v, 0);
    assert!(*elt == 3, 10);
    let elt = vector::borrow_mut(&mut v, 0);
    assert!(*elt == 3, 10);

    let elt = vector::pop_back(&mut v);
    assert!(elt == 2, 10);
    let elt = vector::pop_back(&mut v);
    assert!(elt == 3, 10);

    vector::destroy_empty(v);
  }

  public fun test_u64() {
    let v: vector<u64> = vector::empty();

    let len = vector::length(&v);
    assert!(len == 0, 10);

    vector::push_back(&mut v, 2);
    vector::push_back(&mut v, 3);

    let len = vector::length(&v);
    assert!(len == 2, 10);

    vector::swap(&mut v, 0, 1);

    let elt = vector::borrow(&v, 0);
    assert!(*elt == 3, 10);
    let elt = vector::borrow_mut(&mut v, 0);
    assert!(*elt == 3, 10);

    let elt = vector::pop_back(&mut v);
    assert!(elt == 2, 10);
    let elt = vector::pop_back(&mut v);
    assert!(elt == 3, 10);

    vector::destroy_empty(v);
  }

  public fun test_u128() {
    let v: vector<u128> = vector::empty();

    let len = vector::length(&v);
    assert!(len == 0, 10);

    vector::push_back(&mut v, 2);
    vector::push_back(&mut v, 3);

    let len = vector::length(&v);
    assert!(len == 2, 10);

    vector::swap(&mut v, 0, 1);

    let elt = vector::borrow(&v, 0);
    assert!(*elt == 3, 10);
    let elt = vector::borrow_mut(&mut v, 0);
    assert!(*elt == 3, 10);

    let elt = vector::pop_back(&mut v);
    assert!(elt == 2, 10);
    let elt = vector::pop_back(&mut v);
    assert!(elt == 3, 10);

    vector::destroy_empty(v);
  }

  public fun test_u256() {
    let v: vector<u256> = vector::empty();

    let len = vector::length(&v);
    assert!(len == 0, 10);

    vector::push_back(&mut v, 2);
    vector::push_back(&mut v, 3);

    let len = vector::length(&v);
    assert!(len == 2, 10);

    vector::swap(&mut v, 0, 1);

    let elt = vector::borrow(&v, 0);
    assert!(*elt == 3, 10);
    let elt = vector::borrow_mut(&mut v, 0);
    assert!(*elt == 3, 10);

    let elt = vector::pop_back(&mut v);
    assert!(elt == 2, 10);
    let elt = vector::pop_back(&mut v);
    assert!(elt == 3, 10);

    vector::destroy_empty(v);
  }
}

script {
  use 0x10::tests;

  fun main() {
    tests::test_u8();
    tests::test_u16();
    tests::test_u64();
    tests::test_u128();
    tests::test_u256();
  }
}
