module 0x42::loops {
  struct Balance has key { v: u64 }
  fun transfer(from: address, to: address, amount: u64) {
      Balance[from].v -= amount;
      Balance[to].v += amount;
  }
}
