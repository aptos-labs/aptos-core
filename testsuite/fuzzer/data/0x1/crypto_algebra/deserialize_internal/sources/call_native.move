module poc::deserialize_internal {
   use velor_std::crypto_algebra::{Self};
   use velor_std::bls12381_algebra::{Fr, FormatFrLsb};

   public entry fun main(_owner: &signer) {
      let element_fr = crypto_algebra::from_u64<Fr>(42);
      let serialized_bytes = crypto_algebra::serialize<Fr, FormatFrLsb>(&element_fr);
      let _deserialized_option = crypto_algebra::deserialize<Fr, FormatFrLsb>(&serialized_bytes);
      assert!(_deserialized_option.is_some());
   }

  #[test(owner=@0x123)]
  fun a(owner:&signer){
     main(owner);
   }
}
