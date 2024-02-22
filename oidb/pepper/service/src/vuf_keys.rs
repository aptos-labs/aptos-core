use aptos_oidb_pepper_common::{vuf, vuf::VUF, VUFVerificationKey};
use once_cell::sync::Lazy;

pub struct VufScheme0Sk {
    pub sk_bytes: Vec<u8>,
}

pub static VUF_SCHEME0_SK: Lazy<Vec<u8>> = Lazy::new(|| {
    let vuf_key_hex =
        std::env::var("VRF_KEY_HEX").expect("VRF_KEY_HEX is required for pepper calculation");
    hex::decode(vuf_key_hex).expect("vrf_key_hex should be a valid hex string")
});

pub static VUF_VERIFICATION_KEY_JSON: Lazy<String> = Lazy::new(|| {
    let pk = vuf::scheme0::Scheme::pk_from_sk(&VUF_SCHEME0_SK).expect("bad sk");
    let obj = VUFVerificationKey {
        scheme_name: vuf::scheme0::Scheme::scheme_name(),
        payload_hexlified: hex::encode(pk),
    };
    serde_json::to_string_pretty(&obj).unwrap()
});
