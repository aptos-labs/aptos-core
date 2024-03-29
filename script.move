script {
    use aptos_framework::jwks;
    use aptos_framework::aptos_governance;
    use std::string::utf8;
    fun main(core_resources: &signer) {
        let framework_signer = aptos_governance::get_signer_testnet_only(core_resources, @0000000000000000000000000000000000000000000000000000000000000001);
        let jwk = jwks::new_rsa_jwk(
            utf8(b"test-rsa"),
            utf8(b"RS256"),
            utf8(b"AQAB"),
            utf8(b"y5Efs1ZzisLLKCARSvTztgWj5JFP3778dZWt-od78fmOZFxem3a_aYbOXSJToRp862do0PxJ4PDMpmqwV5f7KplFI6NswQV-WPufQH8IaHXZtuPdCjPOcHybcDiLkO12d0dG6iZQUzypjAJf63APcadio-4JDNWlGC5_Ow_XQ9lIY71kTMiT9lkCCd0ZxqEifGtnJe5xSoZoaMRKrvlOw-R6iVjLUtPAk5hyUX95LDKxwAR-oshnj7gmATejga2EvH9ozdn3M8Go11PSDa04OQxPcA25OoDTfxLvT28LRpSXrbmUWZ-O_lGtDl3ZAtjIguYGEobTk4N11eRssC95Cw")
        );
        let patches = vector[
            jwks::new_patch_upsert_jwk(b"test.oidc.provider", jwk),
        ];
        jwks::set_patches(&framework_signer, patches);
    }
}
