module poc::length {
   use std::vector;

   public entry fun main(_owner:&signer) {
      let v = vector[1u64, 2u64, 3u64];
      let len = vector::length(&v);
      let _ = len;
   }

  #[test(owner=@0x123)]
  fun a(owner:&signer){
     main(owner);
   }
}
