module poc::sha3_256 {
   use std::hash;

   public entry fun main(_owner: &signer) {
      let data = vector[1u8, 2u8, 3u8];
      let _hash = hash::sha3_256(data);
   }

  #[test(owner=@0x123)]
  fun a(owner:&signer){
     main(owner);
   }
}
