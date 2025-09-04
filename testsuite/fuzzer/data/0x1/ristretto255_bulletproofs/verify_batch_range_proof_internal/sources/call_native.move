module poc::verify_batch_range_proof_internal {
    use velor_std::ristretto255_bulletproofs::{Self, range_proof_from_bytes};
    use velor_std::ristretto255::{RistrettoPoint, new_compressed_point_from_bytes, point_decompress, basepoint, hash_to_point_base};
    use std::option::{Self};

    const A_DST: vector<u8> = b"VelorBulletproofs";
    const A_COMM: vector<u8> = x"0a665260a4e42e575882c2cdcb3d0febd6cf168834f6de1e9e61e7b2e53dbf14";
    const B_COMM: vector<u8> = x"748c244d880a1de3970a3d01670a04db6b74b9741bfec8732e512312384a6515";
    const AB_BATCH_RANGE_PROOF_PEDERSEN: vector<u8> = x"103086c56ead10712514d2807c5605cb5f3a090566196549b5f03bedd7c1f450b4619bca9b00f87b2e039e844c24f9f2512901eea7f8f322f218f58c37186e1bd40ae74942f69b18f6806a536b2ab0793ab8e646eafc6e31d5219545dfcbb21334230c4e063e682d1f37fdfe7258d1735af1ba4764ca182803ef4566ddd386143550b83b8d686514988ee05bb7b4180f3b296a0a9711976365b678b537e2190c49cecded1d209ecec733e5cb85d5427f1f2ef1a44ebac41fdbf822692bd68b012515065faab0611aaabe87c1facbe68e648f2e2a0de6e5e81490dfa178546d0e1ec7a7c7ee6eb1e72f0e62b6a81abf23d4e4f946e5c5b28ca287d7ee30c72667ec1203ea9314a4ef182e3ed8a49700cb2452c3765fd29611e2abb5d8aa1970387452cd473383707a0b8e2eb46ba6826654e03ba5f73b56a0ae30012dc723576e76b280339600decef76eda350232ee9e53b373d745b958a19c8b4e7133f4b846727dab188441bb7d2484a73a9a83c1c94e7bea0ea0253418d3d5a751e63f940106e597772d169a01d93b495d10c08725c5d8cdef24306a164a2e1fa1b19eb0217239bbc661e0f1ead2bf3ecc3f178b6b49c61aa2c45f4832ba9ebc2744b79b413081e824b0978cab1934d29760f77751450e409da17941ff693b7dbc0b45d0659aeca05e1e92572fcd4c4d5846e7963e25cce6d54fc4a963da031747695a8e2000469e22e682e1b3f141891121d189504db63b4ab40e0d4c59f0b945b8188b79f0eb4916723a757bcfc787863ff28c5555c8ad93df81bba7b2ff9c164e180331a8b24cff4a9de0d2a8b71f73d24521781f0ced1a064698af138c00160c87eb7ffca5ab1d9a1bec5144c648c5f51a6093dbe8ed88a2fcaab4d5412c60ebb25827d8cab48787f705c5781e2ecd82939d3b3f864c21701fcecbc57b196db7c055273e86ac654a24016abd8ba7c6e87610a0e1b70ff57378992b2d5d45c963829b0aa9323b0dde3f02382e583cb3733c187b46903ed629820ec8043a8c18df42dc0a";
    const MAX_RANGE_BITS: u64 = 64;
    const UNSUPPORTED_BITS: u64 = 10;
    const SUPPORTED_BATCH_SIZE: u64 = 2;
    const UNSUPPORTED_BATCH_SIZE: u64 = 3;

    public entry fun main(_owner:&signer) {
        let proof = range_proof_from_bytes(AB_BATCH_RANGE_PROOF_PEDERSEN);

        let comms_valid_size = vector<RistrettoPoint>[
            {
                let opt = new_compressed_point_from_bytes(A_COMM);
                assert!(option::is_some(&opt), 1);
                let compressed = option::extract(&mut opt);
                point_decompress(&compressed)
            },
            {
                let opt = new_compressed_point_from_bytes(B_COMM);
                assert!(option::is_some(&opt), 2);
                let compressed = option::extract(&mut opt);
                point_decompress(&compressed)
            }
        ];

        let val_base = basepoint();
        let rand_base = hash_to_point_base();
        let dst = A_DST;
        let num_bits = MAX_RANGE_BITS;

        let result_ok = ristretto255_bulletproofs::verify_batch_range_proof(
            &comms_valid_size,
            &val_base, &rand_base,
            &proof, num_bits, dst
        );
        assert!(result_ok, 3);
    }

   #[test(owner=@0x123)]
   fun a(owner:&signer){
      main(owner);
    }
}
