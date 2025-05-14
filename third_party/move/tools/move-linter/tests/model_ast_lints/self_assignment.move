module 0xc0ffee::m {
  use std::vector;

  struct X has drop, copy { f: u64 }
  struct Y has drop, copy { x1: X, x2: X }
  struct Pair(u64, u8) has copy, drop;

  public fun local() : u64 {
    let x = 2;
    x = x;
    x
  }

  public fun strct() {
    let x1 = X {f : 2};
    x1 = x1;
    x1.f = x1.f;
    let y = Y {x1 : X {f : 4}, x2 : X {f : 2}};
    y.x1.f = y.x1.f;
  }

  public fun vardec() : X {
    let x = 5;
    let x = x;
    let x1 = X {f : x};
    let x1 = x1;
    x1
  }

  #[lint::skip(almost_swapped)]
  public fun references() {
    let y1 = Y {x1 : X {f : 4}, x2 : X {f : 2}};
    let y2 = &mut y1;
    y2 = y2;
    *y2 = *y2;
    (*y2).x2 = (*y2).x2;
    y2.x2 = y2.x2;
  }

  #[lint::skip(almost_swapped)]
  public fun inner_ref() {
    let y1 = Y {x1 : X {f : 4}, x2 : X {f : 2}};
    let y2 = &mut y1;
    (*y2).x2 = (*y2).x2;
    (*y2).x2 = y2.x2;
    y2.x2 = (*y2).x2;
  }

  public fun positional() : Pair {
    let pair = Pair(4, 2);
    pair.0 = pair.0;
    pair = pair;
    pair
  }

  public fun vect() : vector<u64> {
    let v1 = vector::empty<u64>();
    vector::push_back(&mut v1, 4);
    v1 = v1;
    v1
  }

  enum E has drop {
    A { x: u64 },
    B { x: u64 },
  }

  fun enum() {
    let e = E::A { x: 5 };
    e = e;
    e.x = e.x;
  }

  public fun no_self_assign_no_warn() {
    let x : u64;
    let y : u64 = 2;
    x = y;
    let y1 = Y {x1 : X {f : 4}, x2 : X {f : 2}};
    y1.x1 = y1.x2;
    y1.x1.f = 5;
    let y1 = &mut y1;
    y1.x1.f = x;
  }

  public fun out_of_scope_vector_no_warn() {
    let v1 = vector::empty<u64>();
    vector::push_back(&mut v1, 4);
    v1[0] = v1[0];
  }

  #[lint::skip(needless_mutable_reference)]
  fun iden(x : &mut X): &mut X {
    x
  }

  public fun out_of_scope_function_no_warn() {
    let x1 = X {f : 4};
    *iden(&mut x1) = *iden(&mut x1);
  }

  #[lint::skip(self_assignment)]
  public fun test_no_warn() : u64 {
    let y : u64 = 2;
    y = y;
    y
  }

  public fun no_warn_deref(): u64 {
    let x = &3;
    let x = *x;
    x
  }
}
