use std::path::PathBuf;
use aptos_secure_storage::KVStorage;
use crate::{CliCommand, CliTypedResult};
use crate::nft::{CollectionEntry, CollectionsEntry};

#[derive(Debug, Parser)]
pub struct CreateNFT {
    /// path to the the folder containing media files and json files
    #[clap(long, parse(try_from_str=crate::common::types::load_account_arg))]
    pub(crate) asset_folder: PathBuf,
}

#[async_trait]
impl CliCommand<String> for CreateNFT {
    fn command_name(&self) -> &'static str {
        "CreateNFT"
    }

    async fn execute(self) -> CliTypedResult<String> {
        let asset_folder = self.asset_folder;
    }
}

fn get_field_or_error(json_value: serde_json::value , name: String) -> serde_json::value {
    json_value
        .get(name)
        .expect(format!("{} is missing from the input JSON file", name).as_str())
}

fn parse_json_input_file(json_file: PathBuf) -> CollectionsEntry {
    // parse the json file to
    let content = std::fs::read_to_string(json_file.as_path())
        .expect("cannot read NFT input file");
    let raw_data = serde_json::from_str(content.as_str())
        .expect("cannot parse NFT input file as json");

    let mut collections_entry = CollectionsEntry {
        collections: Vec::new()
    };
    let collections = get_field_or_error(raw_data, "collections".to_string());
    for col in collections {
        let collection = parse_json_collection(col);
        collections_entry.collections.push(collection);
    };
    collections_entry
}

fn parse_json_collection(collection: serde_json::value) -> CollectionEntry {

}
