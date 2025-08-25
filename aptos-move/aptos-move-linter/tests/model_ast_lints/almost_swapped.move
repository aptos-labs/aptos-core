module 0xc0ffee::m {
  use std::vector;

  struct X has drop, copy { f: u64 }
  struct Y has drop, copy { x1: X, x2: X }
  struct Pair(u64, u8) has copy, drop;

  public fun assign_assign_swap_var() {
    let x : u64;
    let y : u64 = 2;
    x = y;
    y = x;
    if (y == x) ();
  }

  public fun assign_assign_swap_struct() {
    let x1 : X;
    let x2 = X {f : 2};
    x1 = x2;
    x2 = x1;
    if (x2 == x1) ();
  }

  public fun mutate_mutate_swap_struct() {
    let x1 = X {f : 4};
    let x2 = X {f : 2};
    x1.f = x2.f;
    x2.f = x1.f;
  }

  public fun mutate_mutate_swap_struct_deep() {
    let y1 = Y {x1 : X {f : 4}, x2 : X {f : 2}};
    let y2 = Y {x1 : X {f : 1}, x2 : X {f : 3}};
    y1.x1 = y2.x1;
    y2.x1 = y1.x1;
    y1.x1.f = y2.x1.f;
    y2.x1.f = y1.x1.f;
  }

  public fun assign_mutate_swap() {
    let y1 = Y {x1 : X {f : 4}, x2 : X {f : 2}};
    let y2 : Y;
    let y3 = &mut y1;
    y2 = *y3;
    *y3 = y2;
  }

  public fun mutate_assign_swap() {
    let y1 = Y {x1 : X {f : 4}, x2 : X {f : 2}};
    let y2 = Y {x1 : X {f : 4}, x2 : X {f : 2}};
    let y3 = &mut y1;
    *y3 = y2;
    y2 = *y3;
    if (y2 == y1) ();
  }

  public fun inner_swap() {
    let y1 = Y {x1 : X {f : 4}, x2 : X {f : 2}};
    let y2 = Y {x1 : X {f : 4}, x2 : X {f : 2}};
    let y3 = &y1;
    (*y3).x2 = y2.x2;
    y2.x2 = (*y3).x2;
  }

  public fun inner_deref() {
    let y1 = Y {x1 : X {f : 4}, x2 : X {f : 2}};
    let y2 = Y {x1 : X {f : 4}, x2 : X {f : 2}};
    let y3 = &mut y1;
    let y4 = &mut y2;
    (*y3).x2 = (*y4).x2;
    y4.x2 = y3.x2;
    (*y3) = *y4;
    y4 = y3;
    if (y4 == y3) ()
  }

  public fun positional_struct_swap() {
    let pair1 = Pair(4, 2);
    let pair2 = Pair(1, 3);
    pair1.0 = pair2.0;
    pair2.0 = pair1.0;
  }

  public fun double_swap() {
    let x : u64;
    let y : u64 = 2;
    x = y;
    y = x;
    x = y;
    if (y == x) ();
  }

  public fun entire_vector_swap() {
    let v1 = vector::empty<u64>();
    vector::push_back(&mut v1, 4);
    let v2 = vector::empty<u64>();
    vector::push_back(&mut v2, 2);
    v1 = v2;
    v2 = v1;
    if (v1 == v2) ();
  }

  enum E has drop {
    A { x: u64 },
    B { x: u64 },
  }

  fun enum_variant_swap() {
    let e = E::A { x: 5 };
    let f : u64;
    f = e.x;
    e.x = f;
  }

  public fun no_swap_no_warn() {
    let x : u64;
    let y : u64 = 2;
    x = y;
    y = x + 1;
    if (y == x) ();
    let y1 = Y {x1 : X {f : 4}, x2 : X {f : 2}};
    let y2 = Y {x1 : X {f : 1}, x2 : X {f : 3}};
    y1.x1 = y2.x1;
    y2.x1 = y1.x2;
  }

  public fun out_of_scope_vector_swap_no_warn() {
    let v1 = vector::empty<u64>();
    vector::push_back(&mut v1, 4);
    let v2 = vector::empty<u64>();
    vector::push_back(&mut v2, 2);
    v1[0] = v2[0];
    v2[0] = v1[0];
  }

  public fun out_of_scope_vector_swap_no_warn_2(): vector<u64> {
    let v = vector[1, 2];
    v[0] = v[1];
    v[1] = v[0];
    v
  }

  #[lint::skip(needless_mutable_reference)]
  fun iden(x : &mut X): &mut X {
    x
  }

  public fun out_of_scope_function_swap_no_warn() {
    let x1 = X {f : 4};
    let x2 = X {f : 2};
    *iden(&mut x1) = x2;
    x2 = *iden(&mut x1);
    if (x2 == x1) ();
  }

  #[lint::skip(almost_swapped)]
  public fun test_no_warn() {
    let x : u64;
    let y : u64 = 2;
    x = y;
    y = x;
    if (y == x) ();
  }
}
