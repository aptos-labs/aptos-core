use std::{env, fs, path::Path};

const IMPORTED_MAINNET_TXNS: &str = "imported_mainnet_txns";
const IMPORTED_TESTNET_TXNS: &str = "imported_testnet_txns";
const SCRIPTED_TRANSACTIONS_TXNS: &str = "scripted_transactions";
#[derive(Default)]
pub struct TransactionCodeBuilder {
    transactions_code: String,
    name_function_code: String,
}

impl TransactionCodeBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_directory(mut self, dir_name: &str, module_name: &str, generate_name_function: bool) -> Self {
        let json_dir = Path::new("json_transactions").join(dir_name);
        let mut all_constants = String::new();

        for entry in fs::read_dir(json_dir).expect("Failed to read directory") {
            let entry = entry.expect("Failed to get directory entry");
            let path = entry.path();

            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                let file_name = path.file_stem().unwrap().to_str().unwrap();
                let const_name = format!(
                    "{}_{}",
                    module_name.to_uppercase(),
                    file_name.to_uppercase().replace('-', "_")
                );

                self.transactions_code.push_str(&format!(
                    r#"
                    pub const {const_name}: &[u8] = include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/json_transactions/{dir_name}/{file_name}.json"));
                    "#,
                    const_name = const_name,
                    dir_name = dir_name,
                    file_name = file_name,
                ));

                all_constants.push_str(&format!("{},", const_name));

                if generate_name_function {
                    self.name_function_code.push_str(&format!(
                        "        {const_name} => Some(\"{file_name}\"),\n",
                        const_name = const_name,
                        file_name = file_name
                    ));
                }
            }
        }

        if !all_constants.is_empty() {
            self.transactions_code.push_str(&format!(
                "pub const ALL_{}: &[&[u8]] = &[{}];\n",
                module_name.to_uppercase(),
                all_constants
            ));
        }

        self
    }

    pub fn add_transaction_name_function(mut self) -> Self {
        if !self.name_function_code.is_empty() {
            self.transactions_code.push_str(
                r#"
                pub fn get_transaction_name(const_data: &[u8]) -> Option<&'static str> {
                    match const_data {
                "#,
            );

            self.transactions_code.push_str(&self.name_function_code);

            self.transactions_code.push_str(
                r#"
                    _ => None,
                }
            }
            "#,
            );
        }
        self
    }

    pub fn build(self) -> String {
        self.transactions_code
    }
}

fn main() {
    let out_dir = env::var("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("generate_transactions.rs");

    // Create necessary directories if missing
    create_directory_if_missing(&format!("json_transactions/{}", IMPORTED_MAINNET_TXNS));
    create_directory_if_missing(&format!("json_transactions/{}", IMPORTED_TESTNET_TXNS));
    create_directory_if_missing(&format!("json_transactions/{}", SCRIPTED_TRANSACTIONS_TXNS));

    // Using the builder pattern to construct the code
    let code = TransactionCodeBuilder::new()
        .add_directory(IMPORTED_MAINNET_TXNS, IMPORTED_MAINNET_TXNS, false)
        .add_directory(IMPORTED_TESTNET_TXNS, IMPORTED_TESTNET_TXNS,false)
        .add_directory(SCRIPTED_TRANSACTIONS_TXNS, SCRIPTED_TRANSACTIONS_TXNS, true)
        .add_transaction_name_function()
        .build();

    fs::write(dest_path, code).unwrap();
}

// Helper function to create directories if they are missing
fn create_directory_if_missing(dir: &str) {
    let path = Path::new(dir);
    if !path.exists() {
        fs::create_dir_all(path).expect("Failed to create directory");
    }
}
