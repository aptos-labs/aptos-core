// Copyright © Aptos Foundation

fn main() {
    #[cfg(feature = "generate")]
    {
        let config = prost_build::Config::new();
        generate(config, "src/grpc");
    }
}

#[cfg(feature = "generate")]
fn generate(config: prost_build::Config, out_dir: impl AsRef<std::path::Path>) {
    tonic_build::configure()
        .build_server(false)
        .out_dir(out_dir)
        .compile_with_config(
            config,
            &["googleapis/google/pubsub/v1/pubsub.proto"],
            &["googleapis"],
        )
        .unwrap();
}
