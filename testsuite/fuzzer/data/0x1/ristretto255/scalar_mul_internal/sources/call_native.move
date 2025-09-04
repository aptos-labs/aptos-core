module poc::scalar_mul_internal {
    use velor_std::ristretto255;

    public entry fun main(_owner:&signer) {
        let s1 = ristretto255::new_scalar_from_u64(3u64);
        let s2 = ristretto255::new_scalar_from_u64(4u64);
        let _result_scalar = ristretto255::scalar_mul(&s1, &s2);
    }

    #[test(owner=@0x123)]
    fun a(owner:&signer){
       main(owner);
    }
}
