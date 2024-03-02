



use super::bits::Bits;






pub fn jwt_bit_len(jwt: &str) -> usize {
    jwt.len()*8
}


/// input: jwt as base64 without padding.
/// output: length of bit representation of jwt, encoded in big-endian as 8 bits.
pub fn jwt_bit_len_binary(jwt_unsigned: &str) -> Bits {
    let L = jwt_bit_len(jwt_unsigned);
    let L_binary_BE = Bits::raw(&format!("{L:064b}"));

    L_binary_BE
}






/// input: jwt as base64 without padding.
/// output: bit representation of sha padding
pub fn compute_sha_padding(jwt_unsigned: &str) -> Bits {
    let mut padding_bits = Bits::new();
    let L = jwt_bit_len(jwt_unsigned);
    // Following the spec linked here:
    //https://www.rfc-editor.org/rfc/rfc4634.html#section-4.1
    // Step 4.1.a: add bit '1' 
    padding_bits += Bits::raw("1");
    // Step 4.1.b Append K '0' bits where K is the smallest non-negative integer solution to L+1+K = 448 mod 512, and L is the length of the message in bits
    let K = 448 - (L % 512) - 1;
    padding_bits += Bits::raw(&("0".repeat(K)));
    // 4.1.c Append L in binary form (big-endian) as 64 bits
    padding_bits += jwt_bit_len_binary(jwt_unsigned);

    padding_bits
}




pub fn compute_sha_padding_without_len(jwt_unsigned: &str) -> Bits {
    let mut padding_bits = Bits::new();
    let L = jwt_bit_len(jwt_unsigned);
    // Following the spec linked here:
    //https://www.rfc-editor.org/rfc/rfc4634.html#section-4.1
    // Step 4.1.a: add bit '1' 
    padding_bits += Bits::raw("1");
    // Step 4.1.b Append K '0' bits where K is the smallest non-negative integer solution to L+1+K = 448 mod 512, and L is the length of the message in bits
    let K = 448 - (L % 512) - 1;
    padding_bits += Bits::raw(&("0".repeat(K)));
    // Skip 4.1.c 

    padding_bits
}



pub fn with_sha_padding_bytes(jwt_unsigned: &str) -> Vec<u8> {
        (Bits::bit_representation_of_str(jwt_unsigned) + compute_sha_padding(jwt_unsigned)).as_bytes().expect("Should have length a multiple of 8")
}





#[cfg(test)]
mod tests {
    use crate::input_conversion::encoding::{JwtParts, FromB64};
    use crate::input_conversion::sha::with_sha_padding_bytes;

    #[test]
    fn test_compute_sha_padding() {
        let jwt = JwtParts::from_b64("eyJhbGciOiJSUzI1NiIsImtpZCI6InRlc3RfandrIiwidHlwIjoiSldUIn0.eyJpc3MiOiJodHRwczovL2FjY291bnRzLmdvb2dsZS5jb20iLCJhenAiOiI0MDc0MDg3MTgxOTIuYXBwcy5nb29nbGV1c2VyY29udGVudC5jb20iLCJhdWQiOiI0MDc0MDg3MTgxOTIuYXBwcy5nb29nbGV1c2VyY29udGVudC5jb20iLCJzdWIiOiIxMTM5OTAzMDcwODI4OTk3MTg3NzUiLCJoZCI6ImFwdG9zbGFicy5jb20iLCJlbWFpbCI6Im1pY2hhZWxAYXB0b3NsYWJzLmNvbSIsImVtYWlsX3ZlcmlmaWVkIjp0cnVlLCJhdF9oYXNoIjoiYnhJRVN1STU5SW9aYjVhbENBU3FCZyIsIm5hbWUiOiJNaWNoYWVsIFN0cmFrYSIsInBpY3R1cmUiOiJodHRwczovL2xoMy5nb29nbGV1c2VyY29udGVudC5jb20vYS9BQ2c4b2NKdlk0a1ZVQlJ0THhlMUlxS1dMNWk3dEJESnpGcDlZdVdWWE16d1BwYnM9czk2LWMiLCJnaXZlbl9uYW1lIjoiTWljaGFlbCIsImZhbWlseV9uYW1lIjoiU3RyYWthIiwibG9jYWxlIjoiZW4iLCJpYXQiOjE3MDAyNTU5NDQsImV4cCI6MjcwMDI1OTU0NCwibm9uY2UiOiI5Mzc5OTY2MjUyMjQ4MzE1NTY1NTA5NzkwNjEzNDM5OTAyMDA1MTU4ODcxODE1NzA4ODczNjMyNDMxNjk4MTkzNDIxNzk1MDMzNDk4In0.000").unwrap();
        let expected_decoded = "{\"iss\":\"https://accounts.google.com\",\"azp\":\"407408718192.apps.googleusercontent.com\",\"aud\":\"407408718192.apps.googleusercontent.com\",\"sub\":\"113990307082899718775\",\"hd\":\"aptoslabs.com\",\"email\":\"michael@aptoslabs.com\",\"email_verified\":true,\"at_hash\":\"bxIESuI59IoZb5alCASqBg\",\"name\":\"Michael Straka\",\"picture\":\"https://lh3.googleusercontent.com/a/ACg8ocJvY4kVUBRtLxe1IqKWL5i7tBDJzFp9YuWVXMzwPpbs=s96-c\",\"given_name\":\"Michael\",\"family_name\":\"Straka\",\"locale\":\"en\",\"iat\":1700255944,\"exp\":2700259544,\"nonce\":\"9379966252248315565509790613439902005158871815708873632431698193421795033498\"}";
        let expected_with_sha_padding_bytes : [u8; 896] = [101, 121, 74, 104, 98, 71, 99, 105, 79, 105, 74, 83, 85, 122, 73,
        49, 78, 105, 73, 115, 73, 109, 116, 112, 90, 67, 73, 54, 73, 110, 82, 108, 99, 51, 82, 102,
        97, 110, 100, 114, 73, 105, 119, 105, 100, 72, 108, 119, 73, 106, 111, 105, 83, 108, 100,
        85, 73, 110, 48, 46, 101, 121, 74, 112, 99, 51, 77, 105, 79, 105, 74, 111, 100, 72, 82,
        119, 99, 122, 111, 118, 76, 50, 70, 106, 89, 50, 57, 49, 98, 110, 82, 122, 76, 109, 100,
        118, 98, 50, 100, 115, 90, 83, 53, 106, 98, 50, 48, 105, 76, 67, 74, 104, 101, 110, 65,
        105, 79, 105, 73, 48, 77, 68, 99, 48, 77, 68, 103, 51, 77, 84, 103, 120, 79, 84, 73, 117,
        89, 88, 66, 119, 99, 121, 53, 110, 98, 50, 57, 110, 98, 71, 86, 49, 99, 50, 86, 121, 89,
        50, 57, 117, 100, 71, 86, 117, 100, 67, 53, 106, 98, 50, 48, 105, 76, 67, 74, 104, 100, 87,
        81, 105, 79, 105, 73, 48, 77, 68, 99, 48, 77, 68, 103, 51, 77, 84, 103, 120, 79, 84, 73,
        117, 89, 88, 66, 119, 99, 121, 53, 110, 98, 50, 57, 110, 98, 71, 86, 49, 99, 50, 86, 121,
        89, 50, 57, 117, 100, 71, 86, 117, 100, 67, 53, 106, 98, 50, 48, 105, 76, 67, 74, 122, 100,
        87, 73, 105, 79, 105, 73, 120, 77, 84, 77, 53, 79, 84, 65, 122, 77, 68, 99, 119, 79, 68,
        73, 52, 79, 84, 107, 51, 77, 84, 103, 51, 78, 122, 85, 105, 76, 67, 74, 111, 90, 67, 73,
        54, 73, 109, 70, 119, 100, 71, 57, 122, 98, 71, 70, 105, 99, 121, 53, 106, 98, 50, 48, 105,
        76, 67, 74, 108, 98, 87, 70, 112, 98, 67, 73, 54, 73, 109, 49, 112, 89, 50, 104, 104, 90,
        87, 120, 65, 89, 88, 66, 48, 98, 51, 78, 115, 89, 87, 74, 122, 76, 109, 78, 118, 98, 83,
        73, 115, 73, 109, 86, 116, 89, 87, 108, 115, 88, 51, 90, 108, 99, 109, 108, 109, 97, 87,
        86, 107, 73, 106, 112, 48, 99, 110, 86, 108, 76, 67, 74, 104, 100, 70, 57, 111, 89, 88, 78,
        111, 73, 106, 111, 105, 89, 110, 104, 74, 82, 86, 78, 49, 83, 84, 85, 53, 83, 87, 57, 97,
        89, 106, 86, 104, 98, 69, 78, 66, 85, 51, 70, 67, 90, 121, 73, 115, 73, 109, 53, 104, 98,
        87, 85, 105, 79, 105, 74, 78, 97, 87, 78, 111, 89, 87, 86, 115, 73, 70, 78, 48, 99, 109,
        70, 114, 89, 83, 73, 115, 73, 110, 66, 112, 89, 51, 82, 49, 99, 109, 85, 105, 79, 105, 74,
        111, 100, 72, 82, 119, 99, 122, 111, 118, 76, 50, 120, 111, 77, 121, 53, 110, 98, 50, 57,
        110, 98, 71, 86, 49, 99, 50, 86, 121, 89, 50, 57, 117, 100, 71, 86, 117, 100, 67, 53, 106,
        98, 50, 48, 118, 89, 83, 57, 66, 81, 50, 99, 52, 98, 50, 78, 75, 100, 108, 107, 48, 97, 49,
        90, 86, 81, 108, 74, 48, 84, 72, 104, 108, 77, 85, 108, 120, 83, 49, 100, 77, 78, 87, 107,
        51, 100, 69, 74, 69, 83, 110, 112, 71, 99, 68, 108, 90, 100, 86, 100, 87, 87, 69, 49, 54,
        100, 49, 66, 119, 89, 110, 77, 57, 99, 122, 107, 50, 76, 87, 77, 105, 76, 67, 74, 110, 97,
        88, 90, 108, 98, 108, 57, 117, 89, 87, 49, 108, 73, 106, 111, 105, 84, 87, 108, 106, 97,
        71, 70, 108, 98, 67, 73, 115, 73, 109, 90, 104, 98, 87, 108, 115, 101, 86, 57, 117, 89, 87,
        49, 108, 73, 106, 111, 105, 85, 51, 82, 121, 89, 87, 116, 104, 73, 105, 119, 105, 98, 71,
        57, 106, 89, 87, 120, 108, 73, 106, 111, 105, 90, 87, 52, 105, 76, 67, 74, 112, 89, 88, 81,
        105, 79, 106, 69, 51, 77, 68, 65, 121, 78, 84, 85, 53, 78, 68, 81, 115, 73, 109, 86, 52,
        99, 67, 73, 54, 77, 106, 99, 119, 77, 68, 73, 49, 79, 84, 85, 48, 78, 67, 119, 105, 98,
        109, 57, 117, 89, 50, 85, 105, 79, 105, 73, 53, 77, 122, 99, 53, 79, 84, 89, 50, 77, 106,
        85, 121, 77, 106, 81, 52, 77, 122, 69, 49, 78, 84, 89, 49, 78, 84, 65, 53, 78, 122, 107,
        119, 78, 106, 69, 122, 78, 68, 77, 53, 79, 84, 65, 121, 77, 68, 65, 49, 77, 84, 85, 52, 79,
        68, 99, 120, 79, 68, 69, 49, 78, 122, 65, 52, 79, 68, 99, 122, 78, 106, 77, 121, 78, 68,
        77, 120, 78, 106, 107, 52, 77, 84, 107, 122, 78, 68, 73, 120, 78, 122, 107, 49, 77, 68, 77,
        122, 78, 68, 107, 52, 73, 110, 48, 128, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 26, 24];


        let ascii_decoded = jwt.payload_decoded().unwrap();
        assert!(expected_decoded == ascii_decoded);

        let with_sha_padding = with_sha_padding_bytes(&jwt.unsigned_undecoded());
        assert!(with_sha_padding == expected_with_sha_padding_bytes);
        
    }


}
