script {
    use velor_framework::jwks;
    use std::string::utf8;
    fun main(account: &signer) {{
        let iss = b"test.oidc.provider";
        let kid = utf8(b"RSA");
        let alg = utf8(b"RS256");
        let e = utf8(b"AQAB");
        let n = utf8(b"6S7asUuzq5Q_3U9rbs-PkDVIdjgmtgWreG5qWPsC9xXZKiMV1AiV9LXyqQsAYpCqEDM3XbfmZqGb48yLhb_XqZaKgSYaC_h2DjM7lgrIQAp9902Rr8fUmLN2ivr5tnLxUUOnMOc2SQtr9dgzTONYW5Zu3PwyvAWk5D6ueIUhLtYzpcB-etoNdL3Ir2746KIy_VUsDwAM7dhrqSK8U2xFCGlau4ikOTtvzDownAMHMrfE7q1B6WZQDAQlBmxRQsyKln5DIsKv6xauNsHRgBAKctUxZG8M4QJIx3S6Aughd3RZC4Ca5Ae9fd8L8mlNYBCrQhOZ7dS0f4at4arlLcajtw");
        jwks::update_federated_jwk_set(
            account,
            iss,
            vector[kid],
            vector[alg],
            vector[e],
            vector[n]
        );
    }}
}
