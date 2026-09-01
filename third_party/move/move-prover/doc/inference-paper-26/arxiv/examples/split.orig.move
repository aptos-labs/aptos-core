module 0x42::storage {
  struct Balance has key { v: u64 }
  fun split_balance(dst1: &signer, dst2: &signer, src: address): u64 acquires Balance {
      let Balance { v } = move_from<Balance>(src);
      let half = v / 2;
      move_to(dst1, Balance { v: half });
      move_to(dst2, Balance { v: v - half });
      half
  }
}
