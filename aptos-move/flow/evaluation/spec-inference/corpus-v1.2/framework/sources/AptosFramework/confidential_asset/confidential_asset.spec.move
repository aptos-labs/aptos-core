spec aptos_framework::confidential_asset {
    spec module {
        pragma verify = false;
        // TODO: The Move prover (Boogie) cannot handle `match`/`is` on borrowed enum resources
        // (GlobalConfig::V1/V2). Once this prover limitation is fixed, remove this pragma.
    }
}
