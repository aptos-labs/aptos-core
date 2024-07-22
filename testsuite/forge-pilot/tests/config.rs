use aptos_forge_pilot::config::ForgeConfig;

#[test]
fn deserialize_wrapper_config() {
    let file = std::fs::File::open("tests/forge-wrapper-config.json").unwrap();
    let config: ForgeConfig = serde_json::from_reader(file).unwrap();
    println!("{:?}", config);
}
