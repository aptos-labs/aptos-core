module poc::new_point_from_sha512_internal {
    use aptos_std::ristretto255::{Self, RistrettoPoint};

    public entry fun main(_owner: &signer) {
        let input_bytes = b"some input data";
        let _point: RistrettoPoint = ristretto255::new_point_from_sha512(input_bytes);
    }

    #[test(owner=@0x123)]
    fun a(owner: &signer){
        main(owner);
    }
}
