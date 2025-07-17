


// If we want to use BN254
pub use ark_bn254::{
    Bn254 as PairingSetting, 
    g1::Config as G1Config,
    Fr, 
    G1Affine, 
    G2Affine, 
    G1Projective, 
    G2Projective
};


// If we want to use BLS12-381
//pub use ark_bls12_381::{
//    Bls12_381 as PairingSetting, 
//    g1::Config as G1Config,
//    Fr, 
//    G1Affine, 
//    G2Affine, 
//    G1Projective, 
//    G2Projective
//};

