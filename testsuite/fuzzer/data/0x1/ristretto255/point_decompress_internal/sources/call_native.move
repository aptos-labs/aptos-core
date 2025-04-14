module poc::point_decompress_internal {
    use aptos_std::ristretto255::{Self, RistrettoPoint};

    public entry fun main(_owner: &signer) {
        let compressed_point_bytes = ristretto255::basepoint_compressed();
        let _decompressed_point: RistrettoPoint = ristretto255::point_decompress(&compressed_point_bytes);

        let identity_compressed = ristretto255::point_identity_compressed();
        let _identity_decompressed = ristretto255::point_decompress(&identity_compressed);
    }

    #[test(owner=@0x123)]
    fun a(owner:&signer){
        main(owner);
    }
}
