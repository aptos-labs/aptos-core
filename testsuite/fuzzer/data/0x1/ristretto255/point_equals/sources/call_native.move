module poc::point_equals {
    use aptos_std::ristretto255::{Self};

    public entry fun main(_owner: &signer) {
        let point1 = ristretto255::point_identity();
        let point2 = ristretto255::point_identity();
        let point3 = ristretto255::basepoint();

        assert!(ristretto255::point_equals(&point1, &point2), 1);
        assert!(!ristretto255::point_equals(&point1, &point3), 2);
    }

    #[test(owner=@0x123)]
    fun a(owner: &signer){
        main(owner);
    }
}
