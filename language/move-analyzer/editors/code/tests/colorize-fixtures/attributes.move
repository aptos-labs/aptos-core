#[test_only]
#[/* */ test_only /* */]
#[test]
#[expected_failure(abort_code = 1)]
#[test, /* will fail */ expected_failure(abort_code = /* one */ 1)]
#[attribute(
    address_value = /* address */ @0x1,
    byte_value = b"bytes\n" /* bytes */,
    /* hex */ hex_value = x"beef",
    boolean_value = true, // boolean
    number_value = 0x123u8,
)]
