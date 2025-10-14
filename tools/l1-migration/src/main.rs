use anyhow::Result;
use clap::Parser;
use l1_migration::{
    extract_genesis_and_waypoint,
    utils::{decode_network_address, encode_network_address},
};
use std::path::PathBuf;

/// L1 Migration Tool - Extract genesis and waypoint from database
#[derive(Parser)]
#[command(name = "l1-migration")]
#[command(about = "adhoc command for l1 migration")]
#[command(version)]
struct Args {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Parser)]
enum Commands {
    /// Generate waypoint and genesis files from database
    GenerateWaypointGenesis {
        /// Path to the database directory
        db_path: PathBuf,
        /// Destination directory for extracted files
        destination_path: PathBuf,
    },
    /// Network address encoding/decoding tool
    #[command(about = "Convert between multiaddr strings and BCS hex format")]
    NetworkAddress {
        #[command(subcommand)]
        command: NetworkAddressCommands,
    },
}

#[derive(Parser)]
enum NetworkAddressCommands {
    /// Encode multiaddr string to BCS hex
    Encode {
        /// Multiaddr string to encode to BCS hex format
        ///
        /// Example: /dns/validator.example.com/tcp/6180/noise-ik/a1b2c3d4e5f67890abcdef1234567890abcdef1234567890abcdef1234567890/handshake/1
        #[arg(help = "Multiaddr string to encode to BCS hex format")]
        multiaddr: String,
    },
    /// Decode BCS hex to multiaddr string
    Decode {
        /// BCS hex string to decode to multiaddr format
        ///
        /// Example: 0x013f04021576616c696461746f722e6578616d706c652e636f6d0524180720a1b2c3d4e5f67890abcdef1234567890abcdef1234567890abcdef12345678900801
        #[arg(help = "BCS hex string to decode to multiaddr format")]
        hex: String,
    },
}

fn main() -> Result<()> {
    let args = Args::parse();

    match args.command {
        Commands::GenerateWaypointGenesis {
            db_path,
            destination_path,
        } => {
            // Validate that the database path exists
            if !db_path.exists() {
                eprintln!(
                    "Error: Database path '{}' does not exist",
                    db_path.display()
                );
                std::process::exit(1);
            }

            // Create destination directory if it doesn't exist
            if !destination_path.exists() {
                std::fs::create_dir_all(&destination_path)?;
            }

            // Call the extraction function from the module
            let db_path_str = db_path.to_string_lossy();
            let destination_path_str = destination_path.to_string_lossy();

            extract_genesis_and_waypoint(&db_path_str, &destination_path_str)
        },
        Commands::NetworkAddress { command } => match command {
            NetworkAddressCommands::Encode { multiaddr } => encode_network_address(&multiaddr),
            NetworkAddressCommands::Decode { hex } => decode_network_address(&hex),
        },
    }
}
