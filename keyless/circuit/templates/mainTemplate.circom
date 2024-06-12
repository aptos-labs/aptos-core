pragma circom 2.1.3;

include "helpers/base64.circom";
include "helpers/arrays.circom";
include "helpers/misc.circom";
include "helpers/packing.circom";
include "helpers/hashtofield.circom";
include "helpers/sha.circom";
include "helpers/rsa/rsa_verify.circom";
include "helpers/jwt_field_parsing.circom";
include "helpers/rsa/bigint.circom";
include "../node_modules/circomlib/circuits/poseidon.circom";
include "../node_modules/circomlib/circuits/bitify.circom";

// The main Aptos Keyless circuit
template identity(
    maxJWTLen,          // Max byte length of the full base64 JWT with SHA2 padding
    maxJWTHeaderLen,    // Max byte length of the full base64 JWT header with separator
    maxJWTPayloadLen,   // Max byte length of the full base64 JWT payload with SHA2 padding
    maxAudKVPairLen,    // Max byte length of the ASCII aud field
    maxAudNameLen,      // Max byte length of the ASCII aud name    
    maxAudValueLen,     // Max byte length of the ASCII aud value
    maxIssKVPairLen,    // Max byte length of the ASCII iss field
    maxIssNameLen,      // Max byte length of the ASCII iss name
    maxIssValueLen,     // Max byte length of the ASCII iss value
    maxIatKVPairLen,    // Max byte length of the ASCII iat field
    maxIatNameLen,      // Max byte length of the ASCII iat name
    maxIatValueLen,     // Max byte length of the ASCII iat value
    maxNonceKVPairLen,  // Max byte length of the ASCII nonce field
    maxNonceNameLen,    // Max byte length of the ASCII nonce name
    maxNonceValueLen,   // Max byte length of the ASCII nonce value
    maxEVKVPairLen,     // Max byte length of the ASCII email verified field
    maxEVNameLen,       // Max byte length of the ASCII email verified name
    maxEVValueLen,      // Max byte length of the ASCII email verified value
    maxUIDKVPairLen,    // Max byte length of the ASCII uid field
    maxUIDNameLen,      // Max byte length of the ASCII uid name
    maxUIDValueLen,     // Max byte length of the ASCII uid value
    maxEFKVPairLen      // Max byte length of the ASCII extra field
) {

    signal input jwt[maxJWTLen]; // maxJWTLen is in bytes. Base64 format

    signal input jwt_header_with_separator[maxJWTHeaderLen]; // jwt header + '.'
    signal input jwt_payload[maxJWTPayloadLen];


    signal input header_len_with_separator;
    signal input b64_payload_len; 

    ConcatenationCheck(maxJWTLen, maxJWTHeaderLen, maxJWTPayloadLen)(jwt, jwt_header_with_separator, jwt_payload, header_len_with_separator, b64_payload_len);

    var byte_len = 8;

    // Convert jwt bytes into bits for SHA256 hashing
    signal jwt_bits[byte_len*maxJWTLen] <== BytesToBits(maxJWTLen)(jwt);


    signal input jwt_num_sha2_blocks;
    signal input jwt_len_bit_encoded[8]; // 64 bit encoding of the jwt len, in bits
    signal input padding_without_len[64]; // 1 followed by K 0s, where K is the smallest positive integer solution to L + 1 + K = 448, and L is the jwt length in bits. Max length is 512 bits

    Sha2PaddingVerify(maxJWTLen)(jwt, jwt_num_sha2_blocks, header_len_with_separator+b64_payload_len, jwt_len_bit_encoded, padding_without_len);

    var max_num_jwt_blocks = (maxJWTLen*8)\512; // A SHA2 block is 512 bits. '\' performs division rounding up to a whole integer

    // Compute hash of JWT
    signal jwt_sha_hash[256] <== Sha2_256_prepadded_varlen(max_num_jwt_blocks)(jwt_bits, jwt_num_sha2_blocks-1); 

    var dot = SelectArrayValue(maxJWTLen)(jwt, header_len_with_separator-1);

    dot === 46; // '.'

    var signature_len = 32;
    var size_limbs = 64; // The size of limbs used in the big integer implementation leveraged by RSAVerify65537
    // Pack hash bits into 4 64-bit values
    signal packed_jwt_hash[4] <== BitsToFieldElems(256, size_limbs)(jwt_sha_hash); // (input length, byte size of outputs)

    // Verify signature over hash of JWT using modulus input
    signal input signature[signature_len];
    CheckAre64BitLimbs(signature_len)(signature);
    signal input pubkey_modulus[signature_len];
    // RSA verification assumes the signature is less than the pubkey modulus
    signal sig_ok <== BigLessThan(252, signature_len)(signature, pubkey_modulus);
    sig_ok === 1;

    var hash_le[4];
    for (var i = 0; i < 4; i++) {
        hash_le[i] = packed_jwt_hash[3-i];
    }

    RsaVerifyPkcs1v15(size_limbs, signature_len)(signature, pubkey_modulus, hash_le);

    var max_ascii_jwt_payload_len = (3*maxJWTPayloadLen)\4; //TODO: Describe constraints this puts on max payload size. This equation comes from the implementation of Base64Decode - base64 encoding is about 33% larger than the originally encoded data
    signal input jwt_payload_without_sha_padding[maxJWTPayloadLen];
    signal jwt_payload_hash <== HashBytesToFieldWithLen(maxJWTPayloadLen)(jwt_payload, b64_payload_len);

    CheckSubstrInclusionPoly(maxJWTPayloadLen, maxJWTPayloadLen)(jwt_payload, jwt_payload_hash, jwt_payload_without_sha_padding, b64_payload_len, 0); // index is 0

    log("jwt_payload_without_sha_padding: ");
    signal ascii_jwt_payload[max_ascii_jwt_payload_len] <== Base64Decode(max_ascii_jwt_payload_len)(jwt_payload_without_sha_padding);

    signal ascii_payload_len <== Base64DecodedLength(maxJWTPayloadLen)(b64_payload_len);

    signal ascii_jwt_payload_hash <== HashBytesToFieldWithLen(max_ascii_jwt_payload_len)(ascii_jwt_payload, ascii_payload_len);


    signal string_bodies[max_ascii_jwt_payload_len] <== StringBodies(max_ascii_jwt_payload_len)(ascii_jwt_payload);

    // Check aud field is in the JWT
    signal input aud_field[maxAudKVPairLen]; // ASCII
    signal input aud_field_string_bodies[maxAudKVPairLen]; // ASCII
    signal input aud_field_len; // ASCII
    signal input aud_index; // index of aud field in ASCII jwt
    CheckSubstrInclusionPoly(max_ascii_jwt_payload_len, maxAudKVPairLen)(ascii_jwt_payload, ascii_jwt_payload_hash, aud_field, aud_field_len, aud_index); 
    CheckSubstrInclusionPoly(max_ascii_jwt_payload_len, maxAudKVPairLen)(string_bodies, ascii_jwt_payload_hash, aud_field_string_bodies, aud_field_len, aud_index); 

    // Perform necessary checks on aud field
    var aud_name_len = 3;
    signal input aud_value_index;
    signal input aud_colon_index;
    signal input aud_name[maxAudNameLen];
    signal input use_aud_override;
    use_aud_override * (1-use_aud_override) === 0;

    signal aud_value[maxAudValueLen];
    signal input private_aud_value[maxAudValueLen];
    signal input override_aud_value[maxAudValueLen];
    signal input private_aud_value_len;
    signal input override_aud_value_len;
    
    var s = use_aud_override;
    s * (s-1) === 0; // Ensure s = 0 or s = 1
    for (var i = 0; i < maxAudValueLen; i++) {
        aud_value[i] <== (override_aud_value[i]-private_aud_value[i]) * s + private_aud_value[i];
    }
    signal aud_value_len;
    aud_value_len <== (override_aud_value_len-private_aud_value_len) * s + private_aud_value_len;

    ParseJWTFieldWithQuotedValue(maxAudKVPairLen, maxAudNameLen, maxAudValueLen)(aud_field, aud_name, aud_value, aud_field_string_bodies, aud_field_len, aud_name_len, aud_value_index, aud_value_len, aud_colon_index);

    // Check aud name is correct
    var required_aud_name[aud_name_len] = [97, 117, 100]; // aud
    for (var i = 0; i < aud_name_len; i++) {
        aud_name[i] === required_aud_name[i];
    }

    // Check user id field is in the JWT
    signal input uid_field[maxUIDKVPairLen];
    signal input uid_field_string_bodies[maxUIDKVPairLen];
    signal input uid_field_len;
    signal input uid_index;
    CheckSubstrInclusionPoly(max_ascii_jwt_payload_len, maxUIDKVPairLen)(ascii_jwt_payload, ascii_jwt_payload_hash, uid_field, uid_field_len, uid_index);
    CheckSubstrInclusionPoly(max_ascii_jwt_payload_len, maxUIDKVPairLen)(string_bodies, ascii_jwt_payload_hash, uid_field_string_bodies, uid_field_len, uid_index);

    // Perform necessary checks on user id field. Some fields this might be in practice are "sub" or "email"
    signal input uid_name_len;
    signal input uid_value_index;
    signal input uid_value_len;
    signal input uid_colon_index;
    signal input uid_name[maxUIDNameLen];
    signal input uid_value[maxUIDValueLen];

    ParseJWTFieldWithQuotedValue(maxUIDKVPairLen, maxUIDNameLen, maxUIDValueLen)(uid_field, uid_name, uid_value, uid_field_string_bodies, uid_field_len, uid_name_len, uid_value_index, uid_value_len, uid_colon_index);

    // Check extra field is in the JWT
    signal input extra_field[maxEFKVPairLen];
    signal input extra_field_len;
    signal input extra_index;
    signal input use_extra_field;
    use_extra_field * (use_extra_field-1) === 0; // Ensure 0 or 1

    signal ef_passes <== CheckSubstrInclusionPolyBoolean(max_ascii_jwt_payload_len, maxEFKVPairLen)(ascii_jwt_payload, ascii_jwt_payload_hash, extra_field, extra_field_len, extra_index);

    // Fail if use_extra_field = 1 and ef_passes = 0
    signal not_ef_passes <== NOT()(ef_passes);
    signal ef_fail <== AND()(use_extra_field, not_ef_passes);
    ef_fail === 0;

    // Check that ef is not inside a string body
    signal ef_start_char <== SelectArrayValue(max_ascii_jwt_payload_len)(string_bodies, extra_index);
    ef_start_char === 0;

    // Check email verified field
    signal input ev_field[maxEVKVPairLen];
    signal input ev_field_len;
    signal input ev_index;

    var ev_name_len = 14;
    signal input ev_value_index;
    signal input ev_value_len;
    signal input ev_colon_index;
    signal input ev_name[maxEVNameLen];
    signal input ev_value[maxEVValueLen];

    // Boolean truth table for checking whether we should fail on the results of 'EmailVerifiedCheck'
    // and `CheckSubstrInclusionPolyBoolean`. We must fail if the uid name is 'email', and the provided
    // `ev_field` is not in the full JWT according to the substring check
    // uid_is_email | ev_in_jwt | ev_fail
    //     1        |     1     |   1 
    //     1        |     0     |   0
    //     0        |     1     |   1
    //     0        |     0     |   1
    signal uid_is_email <== EmailVerifiedCheck(maxEVNameLen, maxEVValueLen, maxUIDNameLen)(ev_name, ev_value, ev_value_len, uid_name, uid_name_len);
    signal ev_in_jwt <== CheckSubstrInclusionPolyBoolean(max_ascii_jwt_payload_len, maxEVKVPairLen)(ascii_jwt_payload, ascii_jwt_payload_hash, ev_field, ev_field_len, ev_index);
    signal not_ev_in_jwt <== NOT()(ev_in_jwt);
    signal ev_fail <== AND()(uid_is_email, not_ev_in_jwt);
    ev_fail === 0;

    // Need custom logic here because some providers apparently do not follow the OIDC spec and put the email_verified value in quotes
    ParseEmailVerifiedField(maxEVKVPairLen, maxEVNameLen, maxEVValueLen)(ev_field, ev_name, ev_value, ev_field_len, ev_name_len, ev_value_index, ev_value_len, ev_colon_index);

    // Check iss field is in the JWT
    // Note that because `iss_field` is a public input, we assume the verifier will perform correctness checks on it outside of the circuit. 
    signal input iss_field[maxIssKVPairLen];
    signal input iss_field_string_bodies[maxIssKVPairLen];
    signal input iss_field_len;
    signal input iss_index;
    CheckSubstrInclusionPoly(max_ascii_jwt_payload_len, maxIssKVPairLen)(ascii_jwt_payload, ascii_jwt_payload_hash, iss_field, iss_field_len, iss_index);
    CheckSubstrInclusionPoly(max_ascii_jwt_payload_len, maxIssKVPairLen)(string_bodies, ascii_jwt_payload_hash, iss_field_string_bodies, iss_field_len, iss_index);

    // Perform necessary checks on iss field
    var iss_name_len = 3; // iss
    signal input iss_value_index;
    signal input iss_value_len;
    signal input iss_colon_index;
    signal input iss_name[maxIssNameLen];
    signal input iss_value[maxIssValueLen];

    ParseJWTFieldWithQuotedValue(maxIssKVPairLen, maxIssNameLen, maxIssValueLen)(iss_field, iss_name, iss_value, iss_field_string_bodies, iss_field_len, iss_name_len, iss_value_index, iss_value_len, iss_colon_index);

    // Check name of the iss field is correct
    var required_iss_name[iss_name_len] = [105, 115, 115]; // iss
    for (var i = 0; i < iss_name_len; i++) {
        iss_name[i] === required_iss_name[i];
    }

    // Check iat field is in the JWT
    signal input iat_field[maxIatKVPairLen];
    signal input iat_field_len;
    signal input iat_index;
    CheckSubstrInclusionPoly(max_ascii_jwt_payload_len, maxIatKVPairLen)(ascii_jwt_payload, ascii_jwt_payload_hash, iat_field, iat_field_len, iat_index);

    // Perform necessary checks on iat field
    var iat_name_len = 3; // iat
    signal input iat_value_index;
    signal input iat_value_len;
    signal input iat_colon_index;
    signal input iat_name[maxIatNameLen];
    signal input iat_value[maxIatValueLen];

    ParseJWTFieldWithUnquotedValue(maxIatKVPairLen, maxIatNameLen, maxIatValueLen)(iat_field, iat_name, iat_value, iat_field_len, iat_name_len, iat_value_index, iat_value_len, iat_colon_index);

    // Check that iat is not inside a string body
    signal iat_start_char <== SelectArrayValue(max_ascii_jwt_payload_len)(string_bodies, iat_index);
    iat_start_char === 0;

    // Check name of the iat field is correct
    var required_iat_name[iat_name_len] = [105, 97, 116]; // iat
    for (var i = 0; i < iat_name_len; i++) {
        iat_name[i] === required_iat_name[i];
    }
    
    signal iat_field_elem <== ASCIIDigitsToField(maxIatValueLen)(iat_value, iat_value_len);
    
    signal input exp_date;
    signal input exp_delta;
    signal jwt_not_expired <== LessThan(252)([exp_date, iat_field_elem + exp_delta]);
    jwt_not_expired === 1;

    // Check nonce field is in the JWT
    signal input nonce_field[maxNonceKVPairLen];
    signal input nonce_field_string_bodies[maxNonceKVPairLen];
    signal input nonce_field_len;
    signal input nonce_index;
    CheckSubstrInclusionPoly(max_ascii_jwt_payload_len, maxNonceKVPairLen)(ascii_jwt_payload, ascii_jwt_payload_hash, nonce_field, nonce_field_len, nonce_index);
    CheckSubstrInclusionPoly(max_ascii_jwt_payload_len, maxNonceKVPairLen)(string_bodies, ascii_jwt_payload_hash, nonce_field_string_bodies, nonce_field_len, nonce_index);

    // Perform necessary checks on nonce field
    var nonce_name_len = 5; // nonce
    signal input nonce_value_index;
    signal input nonce_value_len;
    signal input nonce_colon_index;
    signal input nonce_name[maxNonceNameLen];
    signal input nonce_value[maxNonceValueLen];

    ParseJWTFieldWithQuotedValue(maxNonceKVPairLen, maxNonceNameLen, maxNonceValueLen)(nonce_field, nonce_name, nonce_value, nonce_field_string_bodies, nonce_field_len, nonce_name_len, nonce_value_index, nonce_value_len, nonce_colon_index);

    // Check name of the nonce field is correct
    var required_nonce_name[nonce_name_len] = [110, 111, 110, 99, 101]; // nonce
    for (var i = 0; i < nonce_name_len; i++) {
        nonce_name[i] === required_nonce_name[i];
    }

    // Calculate nonce
    signal input temp_pubkey[3]; // Represented as 3 elements of up to 31 bytes each to allow for pubkeys of up to 64 bytes each
    signal input temp_pubkey_len; // This is public and checked by the verifier. Included in nonce hash to prevent collisions
    signal input jwt_randomness;
    signal computed_nonce <== Poseidon(6)([temp_pubkey[0], temp_pubkey[1], temp_pubkey[2], temp_pubkey_len, exp_date, jwt_randomness]);
    log("computed nonce is: ", computed_nonce);

    // Check nonce is correct
    signal nonce_field_elem <== ASCIIDigitsToField(maxNonceValueLen)(nonce_value, nonce_value_len);
    
    nonce_field_elem === computed_nonce;

    // Compute the address seed
    signal input pepper;
    signal private_aud_val_hashed <== HashBytesToFieldWithLen(maxAudValueLen)(private_aud_value, private_aud_value_len);
    log("private aud val hash is: ", private_aud_val_hashed);
    signal uid_value_hashed <== HashBytesToFieldWithLen(maxUIDValueLen)(uid_value, uid_value_len);
    log("uid val hash is: ", uid_value_hashed);
    signal uid_name_hashed <== HashBytesToFieldWithLen(maxUIDNameLen)(uid_name, uid_name_len);
    log("uid name hash is: ", uid_name_hashed);
    signal addr_seed <== Poseidon(4)([pepper, private_aud_val_hashed, uid_value_hashed, uid_name_hashed]);
    log("addr seed is: ", addr_seed);

    // Check public inputs are correct 

    signal override_aud_val_hashed <== HashBytesToFieldWithLen(maxAudValueLen)(override_aud_value, override_aud_value_len);
    log("override aud val hash is: ", override_aud_val_hashed);
    signal hashed_jwt_header <== HashBytesToFieldWithLen(maxJWTHeaderLen)(jwt_header_with_separator, header_len_with_separator);
    log("jwt header hash is: ", hashed_jwt_header);
    signal hashed_pubkey_modulus <== Hash64BitLimbsToFieldWithLen(signature_len)(pubkey_modulus, 256); // 256 bytes per signature
    log("pubkey hash is: ", hashed_pubkey_modulus);
    signal hashed_iss_value <== HashBytesToFieldWithLen(maxIssValueLen)(iss_value, iss_value_len);
    log("iss field hash is: ", hashed_iss_value);
    signal hashed_extra_field <== HashBytesToFieldWithLen(maxEFKVPairLen)(extra_field, extra_field_len);
    log("extra field hash is: ", hashed_extra_field);
    signal computed_public_inputs_hash <== Poseidon(14)([temp_pubkey[0], temp_pubkey[1], temp_pubkey[2], temp_pubkey_len, addr_seed, exp_date, exp_delta, hashed_iss_value, use_extra_field, hashed_extra_field, hashed_jwt_header, hashed_pubkey_modulus, override_aud_val_hashed, use_aud_override]);
    log("public inputs hash is: ", computed_public_inputs_hash);
    
    signal input public_inputs_hash;
    public_inputs_hash === computed_public_inputs_hash;
}
