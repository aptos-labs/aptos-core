pragma circom 2.1.3;

include "mainTemplate.circom";

component main { public [public_inputs_hash] } = identity(
    192*8,      // maxJWTLen
    300,        // maxJWTHeaderLen
    192*8-64,   // maxJWTPayloadLen
    140,        // maxAudKVPairLen
    40,         // maxAudNameLen
    120,        // maxAudValueLen
    140,        // maxIssKVPairLen
    40,         // maxIssNameLen
    120,        // maxIssValueLen
    50,         // maxIatKVPairLen
    10,         // maxIatNameLen
    45,         // maxIatValueLen
    105,        // maxNonceKVPairLen
    10,         // maxNonceNameLen
    100,        // maxNonceValueLen
    30,         // maxEVKVPairLen (email_verified field)
    20,         // maxEVNameLen
    10,         // maxEVValueLen
    350,        // maxUIDKVPairLen
    30,         // maxUIDNameLen
    330,        // maxUIDValueLen
    350         // maxEFKVPairLen
);
