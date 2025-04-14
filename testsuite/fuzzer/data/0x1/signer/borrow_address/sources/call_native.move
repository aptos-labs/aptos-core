module poc::borrow_address {
   use std::signer;

   public entry fun main(owner: &signer) {
      let _addr = signer::borrow_address(owner);
   }

  #[test(owner=@0x123)]
  fun a(owner:&signer){
     main(owner);
   }
}
