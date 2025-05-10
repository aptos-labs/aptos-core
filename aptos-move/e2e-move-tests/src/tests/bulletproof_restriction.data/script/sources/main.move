script {
    use aptos_std::ristretto255::point_identity;
    use aptos_std::ristretto255_bulletproofs;

    fun main() {
        let commitments = vector[point_identity(), point_identity()];
        let val_base = point_identity();
        let rand_base = point_identity();

        // Obtained from test `aptos_crypto::unit_tests::bulletproofs_test::test_generated_bulletproof_verifies`.
        let proof = ristretto255_bulletproofs::range_proof_from_bytes(x"88cc71aa9c3efc08b88ec780271e30e00b1707478e44e390ee1628db94309f38226bc6223f84271c55aa226ce59e06400e4c988212653e21031b7c3fc0aa3c0f52411fc2ffae205035255afc50a1b1f38ca1266f8f1c748c6dd4610d3ba5071b28778a25f716e48055e51ebd954cd306d6e97d19d84b6bad5e851f8259f2107fee4891aea8e8b8666edab5df5bb16ea85fd365fb76d46e2de6cc096036ab3400bde3ccc7f2ea44094805f39e5d48b80802fe73b76d6b6071789a5e7c4c21db0064464e37dfb2ea90e116f980fac001e1d116dd70b6497af2ae8450cd86b9770a54ba789ec3c34b0e552289e004a795f7953a835db9c47b7758319eec03b7385bcc6916e191f8f18cb09a8a7a80a3133c2c1c6fb1dfae58296d9d3e066241146f688fa78252d65f4b96c9315dd5112ace02a10e3930c6747d0cafaa5eaba0110da64a6a6319f707306475b32f0efdd2ce981adbe52e0cdfadbb14f91c41f4387a409df28f0a8f75207f620850fa10a96a50e62debe3a4a3e2b680ab3d9d8e3f21a457eb7fd03161b5ea405faf84bcb89047bc908fd593f2c9dbba7d1c660ee2782c9f5449abb11653a04223a7f245d07a7fc72cf36a458ec7cde4fb6234dcc96a6ad41802fa21d98b4468ade903f0c0588b29febe0ae100a9e091561282ccca619e9a460bb7c6f127b8fa1572010e959b63529e174e09a3ef4cbc5ffe4e5cd62b28aaa80ae2afca74220e0c8c67535061c58de33997c6ccb1ee9d970da085b81d9013556979fedd06ecde6c2353e2fd70c5c036f9181c815379c371dfc9997b11ae1f0f64d758ef64f114753eb40e74e955ccc29cb5bc9974e15c9273127fe412b74c380645c724b500270b5fd6dbb7435c4d6d0abe6a5fa4caf84efab1677508074de2e3e799bc298295e8abacb2288db9a26771d7122b1fd5aa29a629b5b306");
        let num_bits = 16;
        let dst = b"TAG1";
        ristretto255_bulletproofs::verify_batch_range_proof(
            &commitments,
            &val_base,
            &rand_base,
            &proof,
            num_bits,
            dst,
        );
    }
}
