use aptos_forge_pilot::config::ForgeConfig;
use clap::Parser;

#[derive(Parser, Debug)]
struct Args {
    #[clap(short, long, group = "config")]
    config_path: Option<String>,

    #[clap(short, long, group = "config")]
    s3_path: Option<String>,

    #[clap(short, long, group = "config")]
    gcs_path: Option<String>,
}

fn main() {
    let args = Args::parse();
    let config = if let Some(path) = args.config_path {
        ForgeConfig::read_from_file(&path)
    } else if let Some(path) = args.s3_path {
        ForgeConfig::read_from_s3(&path)
    } else if let Some(path) = args.gcs_path {
        ForgeConfig::read_from_gcs(&path)
    } else {
        panic!("No config path provided");
    };

    println!("{:?}", config);
}
