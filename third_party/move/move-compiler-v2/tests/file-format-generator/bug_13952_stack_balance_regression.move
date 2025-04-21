// Test relates to #13952 since the fix made this case fail first
module 0x42::m {

  enum Data has drop {
    V1{x: u64},
    V2{x: u64, y: bool}
    V3
  }

  fun test_v1(): bool {
    let d = Data::V1{x: 43};
    (d is V1)
  }

  fun test_v1v3(): bool {
    let d = Data::V1{x: 43};
    let t = (d is V1|V3);
    let d = Data::V3{};
    t && (d is V1|V3)
  }
}
