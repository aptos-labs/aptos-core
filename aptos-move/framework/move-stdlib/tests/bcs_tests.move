#[test_only]
module std::bcs_tests {
    use std::bcs;
    use std::vector;

    struct Box<T> has copy, drop, store { x: T }
    struct Box3<T> has copy, drop, store { x: Box<Box<T>> }
    struct Box7<T> has copy, drop, store { x: Box3<Box3<T>> }
    struct Box15<T> has copy, drop, store { x: Box7<Box7<T>> }
    struct Box31<T> has copy, drop, store { x: Box15<Box15<T>> }
    struct Box63<T> has copy, drop, store { x: Box31<Box31<T>> }
    struct Box127<T> has copy, drop, store { x: Box63<Box63<T>> }

    #[test]
    fun bcs_bool() {
        let expected_bytes = x"01";
        let actual_bytes = bcs::to_bytes(&true);
        assert!(actual_bytes == expected_bytes, 0);

        let expected_size = vector::length(&actual_bytes);
        let actual_size = bcs::serialized_size(&true);
        assert!(actual_size == expected_size, 1);
    }

    #[test]
    fun bcs_u8() {
        let expected_bytes = x"01";
        let actual_bytes = bcs::to_bytes(&1u8);
        assert!(actual_bytes == expected_bytes, 0);

        let expected_size = vector::length(&actual_bytes);
        let actual_size = bcs::serialized_size(&1u8);
        assert!(actual_size == expected_size, 1);
    }

    #[test]
    fun bcs_u64() {
        let expected_bytes = x"0100000000000000";
        let actual_bytes = bcs::to_bytes(&1);
        assert!(actual_bytes == expected_bytes, 0);

        let expected_size = vector::length(&actual_bytes);
        let actual_size = bcs::serialized_size(&1);
        assert!(actual_size == expected_size, 1);
    }

    #[test]
    fun bcs_u128() {
        let expected_bytes = x"01000000000000000000000000000000";
        let actual_bytes = bcs::to_bytes(&1u128);
        assert!(actual_bytes == expected_bytes, 0);

        let expected_size = vector::length(&actual_bytes);
        let actual_size = bcs::serialized_size(&1u128);
        assert!(actual_size == expected_size, 1);
    }

    #[test]
    fun bcs_vec_u8() {
        let v = x"0f";

        let expected_bytes = x"010f";
        let actual_bytes = bcs::to_bytes(&v);
        assert!(actual_bytes == expected_bytes, 0);

        let expected_size = vector::length(&actual_bytes);
        let actual_size = bcs::serialized_size(&v);
        assert!(actual_size == expected_size, 1);
    }

    fun box3<T>(x: T): Box3<T> {
        Box3 { x: Box { x: Box { x } } }
    }

    fun box7<T>(x: T): Box7<T> {
        Box7 { x: box3(box3(x)) }
    }

    fun box15<T>(x: T): Box15<T> {
        Box15 { x: box7(box7(x)) }
    }

    fun box31<T>(x: T): Box31<T> {
        Box31 { x: box15(box15(x)) }
    }

    fun box63<T>(x: T): Box63<T> {
        Box63 { x: box31(box31(x)) }
    }

    fun box127<T>(x: T): Box127<T> {
        Box127 { x: box63(box63(x)) }
    }

    #[test]
    fun encode_128() {
        let box = box127(true);

        let bytes = bcs::to_bytes(&box);
        let expected_size = vector::length(&bytes);

        let actual_size = bcs::serialized_size(&box);
        assert!(actual_size == expected_size, 0);
    }
}
