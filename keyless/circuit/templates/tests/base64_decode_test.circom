pragma circom 2.1.3;

include "helpers/base64.circom";

template base64_decode_test(maxJWTPayloadLen) {
    var max_ascii_jwt_payload_len = (3*maxJWTPayloadLen)\4;
    signal input jwt_payload[maxJWTPayloadLen];
    signal input ascii_jwt_payload[max_ascii_jwt_payload_len];
    component base64decode = Base64Decode(max_ascii_jwt_payload_len);
    base64decode.in <== jwt_payload;
    ascii_jwt_payload === base64decode.out;

}

component main = base64_decode_test(
    192*8-64   // maxJWTPayloadLen
);
