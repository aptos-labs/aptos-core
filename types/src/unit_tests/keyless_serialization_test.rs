use crate::{
    keyless::{test_utils, Groth16ZkpAndStatement, Pepper},
    transaction::authenticator::EphemeralPublicKey,
};
use aptos_crypto::{
    ed25519::Ed25519PrivateKey,
    traits::{PrivateKey, Uniform},
};

#[test]
fn test_epk_serialization() {
    let ed25519_pk = Ed25519PrivateKey::generate_for_testing().public_key();
    let epk = EphemeralPublicKey::Ed25519 {
        public_key: ed25519_pk,
    };

    assert_eq!(
        serde_json::from_str::<EphemeralPublicKey>(&serde_json::to_string(&epk).unwrap()).unwrap(),
        epk
    );
    assert_eq!(
        bcs::from_bytes::<EphemeralPublicKey>(&bcs::to_bytes(&epk).unwrap()).unwrap(),
        epk
    );

    // these values were generated as follows:
    //println!("{:?}", serde_json::to_string(&epk).unwrap());
    //println!("{:?}", bcs::to_bytes(&epk).unwrap());
    let epk_str = "\"002020fdbac9b10b7587bba7b5bc163bce69e796d71e4ed44c10fcb4488689f7a144\"";
    let epk_bytes = [
        0, 32, 32, 253, 186, 201, 177, 11, 117, 135, 187, 167, 181, 188, 22, 59, 206, 105, 231,
        150, 215, 30, 78, 212, 76, 16, 252, 180, 72, 134, 137, 247, 161, 68,
    ];

    assert_eq!(
        serde_json::to_string(&serde_json::from_str::<EphemeralPublicKey>(epk_str).unwrap())
            .unwrap()
            .as_str(),
        epk_str
    );
    assert_eq!(
        bcs::to_bytes(&bcs::from_bytes::<EphemeralPublicKey>(&epk_bytes).unwrap()).unwrap(),
        epk_bytes
    );
}

#[test]
fn test_pepper_serialization() {
    let pepper = test_utils::get_sample_pepper();

    assert_eq!(
        serde_json::from_str::<Pepper>(&serde_json::to_string(&pepper).unwrap()).unwrap(),
        pepper
    );
    assert_eq!(
        bcs::from_bytes::<Pepper>(&bcs::to_bytes(&pepper).unwrap()).unwrap(),
        pepper
    );

    // these values were generated as follows:
    //println!("{:?}", serde_json::to_string(&pepper).unwrap());
    //println!("{:?}", bcs::to_bytes(&pepper).unwrap());
    let pepper_str = "\"2a000000000000000000000000000000000000000000000000000000000000\"";
    let pepper_bytes = [
        42, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0,
    ];

    assert_eq!(
        serde_json::to_string(&serde_json::from_str::<Pepper>(pepper_str).unwrap())
            .unwrap()
            .as_str(),
        pepper_str
    );
    assert_eq!(
        bcs::to_bytes(&bcs::from_bytes::<Pepper>(&pepper_bytes).unwrap()).unwrap(),
        pepper_bytes
    );
}

#[test]
fn test_groth16_zkp_and_statement_serialization() {
    let groth16_zkp_and_statement = test_utils::get_sample_groth16_zkp_and_statement();

    assert_eq!(
        serde_json::from_str::<Groth16ZkpAndStatement>(
            &serde_json::to_string(&groth16_zkp_and_statement).unwrap()
        )
        .unwrap(),
        groth16_zkp_and_statement
    );
    assert_eq!(
        bcs::from_bytes::<Groth16ZkpAndStatement>(
            &bcs::to_bytes(&groth16_zkp_and_statement).unwrap()
        )
        .unwrap(),
        groth16_zkp_and_statement
    );

    // these values were generated as follows:
    //println!("{}", serde_json::to_string(&groth16_zkp_and_statement).unwrap());
    //println!("{:?}", bcs::to_bytes(&groth16_zkp_and_statement).unwrap());
    let groth16_zkp_and_statement_str =
        "{\
          \"proof\":{\
            \"a\":\"0eade6c1a19f5cd304621f3af2d1522a1c0c8da5ea181a1cf724d0cc174fe289\",\
            \"b\":\"cb08df9056241cbad8d2ddccbf9fde31a906a2d979a14a65764190799460232c9ce60313f90f77c9d1aa5da312c5b376bc62573f35fc4e427679328b55c4741d\",\
            \"c\":\"cb6ee083bb9204cb322c6767f33691ee8e980d0465bdd4ebb47675d52e8139ae\"\
          },\
          \"public_inputs_hash\":\"221d09fabd73592c55872e212bce2533d422a814fe3203994ee61d369bef9f2c\"\
        }";
    let groth16_zkp_and_statement_bytes = [
        14, 173, 230, 193, 161, 159, 92, 211, 4, 98, 31, 58, 242, 209, 82, 42, 28, 12, 141, 165,
        234, 24, 26, 28, 247, 36, 208, 204, 23, 79, 226, 137, 203, 8, 223, 144, 86, 36, 28, 186,
        216, 210, 221, 204, 191, 159, 222, 49, 169, 6, 162, 217, 121, 161, 74, 101, 118, 65, 144,
        121, 148, 96, 35, 44, 156, 230, 3, 19, 249, 15, 119, 201, 209, 170, 93, 163, 18, 197, 179,
        118, 188, 98, 87, 63, 53, 252, 78, 66, 118, 121, 50, 139, 85, 196, 116, 29, 203, 110, 224,
        131, 187, 146, 4, 203, 50, 44, 103, 103, 243, 54, 145, 238, 142, 152, 13, 4, 101, 189, 212,
        235, 180, 118, 117, 213, 46, 129, 57, 174, 34, 29, 9, 250, 189, 115, 89, 44, 85, 135, 46,
        33, 43, 206, 37, 51, 212, 34, 168, 20, 254, 50, 3, 153, 78, 230, 29, 54, 155, 239, 159, 44,
    ];

    assert_eq!(
        serde_json::to_string(
            &serde_json::from_str::<Groth16ZkpAndStatement>(groth16_zkp_and_statement_str).unwrap()
        )
        .unwrap()
        .as_str(),
        groth16_zkp_and_statement_str
    );
    assert_eq!(
        bcs::to_bytes(
            &bcs::from_bytes::<Groth16ZkpAndStatement>(&groth16_zkp_and_statement_bytes).unwrap()
        )
        .unwrap(),
        groth16_zkp_and_statement_bytes
    );
}
