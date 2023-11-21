module tournament::token_uris {
    use std::string::{Self, String};

    use tournament::misc_utils;

    friend tournament::token_manager;
    #[test_only] friend tournament::rps_unit_tests;
    #[test_only] friend tournament::test_utils;

    const NUM_TOKEN_URIS: u64 = 250;

    public(friend) fun get_random_token_uri(): String {
        let rand_idx = misc_utils::rand_range(0, NUM_TOKEN_URIS);
        let base_uri = string::utf8(b"https://bafybeicx4i5nkzfmcfd4s7sdid7tlus5hn6t4yog4zdsw63vbmkaxynsfe.ipfs.nftstorage.link/");
        let with_number = misc_utils::concat_any_to_string(base_uri, &rand_idx);
        let with_jpg = misc_utils::join_strings(vector<String> [with_number, string::utf8(b".jpg")], string::utf8(b""));
        with_jpg
    }
}
