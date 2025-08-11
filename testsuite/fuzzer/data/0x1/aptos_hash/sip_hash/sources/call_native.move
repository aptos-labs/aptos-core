module poc::sip_hash {
   use std::aptos_hash;

   public entry fun main(_owner: &signer) {
      let data = vector[1u8, 2u8, 3u8];
      let _hash = aptos_hash::sip_hash(data);
   }

  #[test(owner=@0x123)]
  fun a(owner:&signer){
     main(owner);
   }
}
