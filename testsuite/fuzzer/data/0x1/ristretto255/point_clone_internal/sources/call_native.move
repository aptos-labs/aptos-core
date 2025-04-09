module poc::point_clone_internal {
    use aptos_std::ristretto255;

    public entry fun main(_owner:&signer) {
        let p = ristretto255::point_identity();
        let p_clone = ristretto255::point_clone(&p);
        assert!(ristretto255::point_equals(&p, &p_clone), 0);

        let compressed_p = ristretto255::point_compress(&p);
        let decompressed_p = ristretto255::point_decompress(&compressed_p);
        assert!(ristretto255::point_equals(&p, &decompressed_p), 1);
    }

    #[test(owner=@0x123)]
    fun a(owner:&signer){
       main(owner);
    }
}
