module poc::verify_range_proof_internal {
    use aptos_std::ristretto255_bulletproofs::{Self, range_proof_from_bytes};
    use aptos_std::ristretto255::{Self, new_compressed_point_from_bytes, basepoint, hash_to_point_base};
    use std::option::{Self};

    const A_DST: vector<u8> = b"AptosBulletproofs";
    const A_COMM: vector<u8> = x"0a665260a4e42e575882c2cdcb3d0febd6cf168834f6de1e9e61e7b2e53dbf14";
    const A_RANGE_PROOF_PEDERSEN: vector<u8> = x"d8d422d3fb9511d1942b78e3ec1a8c82fe1c01a0a690c55a4761e7e825633a753cca816667d2cbb716fe04a9c199cad748c2d4e59de4ed04fedf5f04f4341a74ae75b63c1997fd65d5fb3a8c03ad8771abe2c0a4f65d19496c11d948d6809503eac4d996f2c6be4e64ebe2df31102c96f106695bdf489dc9290c93b4d4b5411fb6298d0c33afa57e2e1948c38ef567268a661e7b1c099272e29591e717930a06a2c6e0e2d56aedea3078fd59334634f1a4543069865409eba074278f191039083102a9a0621791a9be09212a847e22061e083d7a712b05bca7274b25e4cb1201c679c4957f0842d7661fa1d3f5456a651e89112628b456026f8ad3a7abeaba3fec8031ec8b0392c0aa6c96205f7b21b0c2d6b5d064bd5bd1a1d91c41625d910688fa0dca35ec0f0e31a45792f8d6a330be970a22e1e0773111a083de893c89419ee7de97295978de90bcdf873a2826746809e64f9143417dbed09fa1c124e673febfed65c137cc45fabda963c96b64645802d1440cba5e58717e539f55f3321ab0c0f60410fba70070c5db500fee874265a343a2a59773fd150bcae09321a5166062e176e2e76bef0e3dd1a9250bcb7f4c971c10f0b24eb2a94e009b72c1fc21ee4267881e27b4edba8bed627ddf37e0c53cd425bc279d0c50d154d136503e54882e9541820d6394bd52ca2b438fd8c517f186fec0649c4846c4e43ce845d80e503dee157ce55392188039a7efc78719107ab989db8d9363b9dfc1946f01a84dbca5e742ed5f30b07ac61cf17ce2cf2c6a49d799ed3968a63a3ccb90d9a0e50960d959f17f202dd5cf0f2c375a8a702e063d339e48c0227e7cf710157f63f13136d8c3076c672ea2c1028fc1825366a145a4311de6c2cc46d3144ae3d2bc5808819b9817be3fce1664ecb60f74733e75e97ca8e567d1b81bdd4c56c7a340ba00";
    const MAX_RANGE_BITS: u64 = 64;
    const UNSUPPORTED_BITS: u64 = 10;

    public entry fun main(_owner:&signer) {
        let proof = range_proof_from_bytes(A_RANGE_PROOF_PEDERSEN);
        let comm_point_option = new_compressed_point_from_bytes(A_COMM);
        assert!(option::is_some(&comm_point_option), 1);
        let comm_compressed = option::extract(&mut comm_point_option);
        let comm_point = ristretto255::point_decompress(&comm_compressed);

        let val_base = basepoint();
        let rand_base = hash_to_point_base();
        let dst = A_DST;
        let num_bits = MAX_RANGE_BITS;

        let result_ok = ristretto255_bulletproofs::verify_range_proof(
            &comm_point,
            &val_base, &rand_base,
            &proof, num_bits, dst
        );
        assert!(result_ok, 2);
    }

   #[test(owner=@0x123)]
   fun a(owner:&signer){
      main(owner);
    }
}
