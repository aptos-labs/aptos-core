module poc::ripemd160_internal {

   public entry fun main(_owner: &signer) {
      let data = vector[1u8, 2u8, 3u8];
      let _hash = aptos_std::aptos_hash::ripemd160(data);
   }

  #[test(owner=@0x123)]
  fun a(owner:&signer){
     main(owner);
   }
}
