module poc::add_internal {
    use velor_std::crypto_algebra::{Self};
    use velor_std::bls12381_algebra::Fr;

    public entry fun main(_owner: &signer) {
        let x = crypto_algebra::from_u64<Fr>(10);
        let y = crypto_algebra::from_u64<Fr>(20);
        let _result = crypto_algebra::add<Fr>(&x, &y);
    }

   #[test(owner=@0x123)]
   fun a(owner:&signer){
      main(owner);
    }
}
