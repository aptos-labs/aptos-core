// TODO: #18199
//# publish
module 0x42::m2 {

}

//# publish
module 0x42::m {

  friend 0x42::m2;

  friend enum Data has drop {
    V1{x: u64},
    V2{x: u64, y: bool}
    V3
  }

  friend enum Data2 has drop {
    V1{x: u64},
    V2{y: u64, x: u64}
    V3
  }

}

//# publish
module 0x42::m2 {

  use 0x42::m::Data;
  use 0x42::m::Data2;

  fun test_v1(): bool {
    let d = Data::V1{x: 43};
    (d is V1) && (&d is V1|V3)
  }

  fun test_v1v3(): bool {
    let d = Data::V1{x: 43};
    let t = (d is V1|V3);
    let d = Data::V3{};
    t && (d is V1|V3)
  }

  fun test_v1v3_ref(): bool {
    let d = Data::V1{x: 43};
    let t = (&d is V1|V3);
    let d = Data::V3{};
    t && (&mut d is V1|V3)
  }

  public fun test_v1_mut_borrow() {
    let d = Data::V1{x: 43};
    let r = &mut d.x;
    *r = 44;
    assert!(d.x == 44, 1);
  }

  public fun test_v2_mut_borrow_2() {
    let d = Data2::V2{y: 43, x: 44};
    let r = &mut d.x;
    *r = 45;
    assert!(d.x == 45, 1);
  }

  public fun test_match_mut_borrow() {
    let d = Data2::V2{y: 43, x: 44};
    match (&mut d) {
      Data2::V2{y, x} if (*y == 43) => {
        *x = 45;
        assert!(d.x == 45, 1);
      },
      _ => {}
    }
  }


}

//# run 0x42::m2::test_v1

//# run 0x42::m2::test_v1v3

//# run 0x42::m2::test_v1v3_ref

//# run 0x42::m2::test_v1_mut_borrow

//# run 0x42::m2::test_v2_mut_borrow_2

//# run 0x42::m2::test_match_mut_borrow
