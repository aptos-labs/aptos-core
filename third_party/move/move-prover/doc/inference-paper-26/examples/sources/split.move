module 0x42::storage {
  struct Balance has key { v: u64 }
  fun split_balance(dst1: &signer, dst2: &signer, src: address): u64 acquires Balance {
      let Balance { v } = move_from<Balance>(src);
      let half = v / 2;
      move_to(dst1, Balance { v: half });
      move_to(dst2, Balance { v: v - half });
      half
  }
  spec split_balance(dst1: &signer, dst2: &signer, src: address): u64 {
      use 0x1::signer;
      pragma opaque = true;
      modifies Balance[src];
      modifies Balance[signer::address_of(dst1)];
      modifies Balance[signer::address_of(dst2)];
      ensures [inferred] result == old(Balance[src]).v / 2;
      ensures [inferred] ..S1 |~ remove<Balance>(src);
      ensures [inferred] S1..S2 |~ publish<Balance>(signer::address_of(dst1), Balance{v: old(Balance[src]).v / 2});
      ensures [inferred] {
          let a = Balance{v: old(Balance[src]).v - old(Balance[src]).v / 2};
          S2.. |~ publish<Balance>(signer::address_of(dst2), a)
      };
      aborts_if [inferred] !exists<Balance>(src);
      aborts_if [inferred] S1 |~ exists<Balance>(signer::address_of(dst1));
      aborts_if [inferred] S2 |~ exists<Balance>(signer::address_of(dst2));
  }

}
