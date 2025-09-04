module poc::native_stack_trace {
   use velor_std::debug;

   public entry fun main(_owner: &signer) {
      debug::print_stack_trace();
   }

  #[test(owner=@0x123)]
  fun a(owner:&signer){
     main(owner);
   }
}
