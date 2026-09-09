module 0x42::loops {
  struct Balance has key { v: u64 }
  fun transfer(from: address, to: address, amount: u64) {
      Balance[from].v -= amount;
      Balance[to].v += amount;
  }
  spec transfer(from: address, to: address, amount: u64) {
      pragma opaque = true;
      modifies Balance[from];
      modifies Balance[to];
      aborts_if [inferred] !exists<Balance>(from);
      aborts_if [inferred] !exists<Balance>(to);
      aborts_if [inferred] Balance[from].v < amount;
      aborts_if [inferred] from != to && Balance[to].v + amount > MAX_U64;
      ensures [inferred] from == to ==> Balance[from] == old(Balance[from]);
      ensures [inferred] from != to ==> Balance[from].v == old(Balance[from]).v - amount;
      ensures [inferred] from != to ==> Balance[to].v == old(Balance[to]).v + amount;
  }

}
