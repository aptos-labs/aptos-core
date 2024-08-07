use aptos_forge_pilot::config::ForgeConfig;

#[test]
fn deserialize_wrapper_config() {
    let file = std::fs::File::open("tests/forge-wrapper-config.json").unwrap();
    let config: ForgeConfig = serde_json::from_reader(file).unwrap();
    println!("{:?}", config);
}

#[test]
fn read_from_file() {
    let config = ForgeConfig::read_from_file("tests/forge-wrapper-config.json");
    println!("{:?}", config);
}

#[cfg(feature = "s3-tests")]
#[test]
fn read_from_s3() {
    let config = ForgeConfig::read_from_s3();
    println!("{:?}", config);
}
