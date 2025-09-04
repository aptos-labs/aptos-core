module poc::point_compress_internal {
    use velor_std::ristretto255::{Self, CompressedRistretto, RistrettoPoint};
    use std::vector;

    public entry fun main(_owner: &signer) {
        let point: RistrettoPoint = ristretto255::basepoint();
        let _compressed_point: CompressedRistretto = ristretto255::point_compress(&point);
        assert!(vector::length(&ristretto255::compressed_point_to_bytes(_compressed_point)) == 32, 1);

        let identity_point = ristretto255::point_identity();
        let _identity_compressed = ristretto255::point_compress(&identity_point);
        assert!(vector::length(&ristretto255::compressed_point_to_bytes(_identity_compressed)) == 32, 2);
    }

    #[test(owner=@0x123)]
    fun a(owner: &signer){
        main(owner);
    }
}
