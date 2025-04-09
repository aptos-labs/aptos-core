module poc::pop_back {
   use std::vector;

   public entry fun main(_owner:&signer) {
      let v = vector[1u64, 2u64, 3u64];
      let elem = vector::pop_back(&mut v);
      let _ = elem;
   }

  #[test(owner=@0x123)]
  fun a(owner:&signer){
     main(owner);
   }
}
