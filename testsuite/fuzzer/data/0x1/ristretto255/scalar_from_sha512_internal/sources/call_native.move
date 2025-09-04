module poc::scalar_from_sha512_internal {
    use velor_std::ristretto255;
    use velor_std::velor_hash;

    public entry fun main(_owner:&signer) {
        let input = b"hello world";
        let hash_digest = velor_hash::sha2_512(input);
        let _scalar = ristretto255::new_scalar_from_sha512(hash_digest);
    }

    #[test(owner=@0x123)]
    fun a(owner:&signer){
       main(owner);
    }
}
