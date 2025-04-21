//# publish
module 0x42::m {

  enum Data has drop {
    V1{x: u64},
    V2{y: bool, x: u64}
    V3{z: u8, x: u64}
  }

  fun test_get_x_v1(): u64 {
    let d = Data::V1{x: 43};
    d.x
  }

  fun test_get_x_v2(): u64 {
    let d = Data::V2{x: 43, y: false};
    d.x
  }

  fun test_get_x_v3(): u64 {
    let d = Data::V3{z: 1, x: 43};
    d.x
  }

  fun test_get_y_v1(): bool {
    let d = Data::V1{x: 43};
    d.y
  }

  fun test_get_y_v2(): bool {
    let d = Data::V2{x: 43, y: true};
    d.y
  }

  fun test_get_y_v3(): bool {
    let d = Data::V3{x: 43, z: 1};
    d.y
  }

}

//# run 0x42::m::test_get_x_v1

//# run 0x42::m::test_get_x_v2

//# run 0x42::m::test_get_x_v3

//# run 0x42::m::test_get_y_v1

//# run 0x42::m::test_get_y_v2

//# run 0x42::m::test_get_y_v3
