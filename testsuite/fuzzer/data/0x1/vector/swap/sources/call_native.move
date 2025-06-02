module poc::swap {
   use std::vector;

   public entry fun main(_owner: &signer) {
      let v = vector[1u64, 2u64, 3u64];
      vector::swap(&mut v, 0, 2);
   }

  #[test(owner=@0x123)]
  fun a(owner:&signer){
     main(owner);
   }
}
