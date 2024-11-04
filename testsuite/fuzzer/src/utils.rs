// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

#[allow(dead_code)]
pub(crate) mod cli {
    use aptos_framework::{BuildOptions, BuiltPackage};
    use aptos_types::{
        account_address::AccountAddress,
        transaction::{EntryFunction, Script, TransactionPayload},
    };
    use move_core_types::{ident_str, language_storage::ModuleId};
    use std::path::PathBuf;

    /// Compiles a Move module from source code.
    /// The compiled module and its metadata are returned serialized.
    /// Those can be used to publish the module on-chain via code_publish_package_txn().
    pub(crate) fn compile_federated_jwk(module_path: &str) -> Result<(), String> {
        let package = BuiltPackage::build(PathBuf::from(module_path), BuildOptions::default())
            .map_err(|e| e.to_string())?;

        let transaction_payload = generate_script_payload_jwk(&package);
        let code_snippet = format!(
            r#"
            let tx = acc
                .transaction()
                .gas_unit_price(100)
                .sequence_number(sequence_number)
                .payload(bcs::from_bytes(&{:?}).unwrap())
                .sign();
            "#,
            bcs::to_bytes(&transaction_payload).unwrap()
        );
        println!("{}", code_snippet);

        Ok(())
    }

    /// Generate a TransactionPayload for modules
    ///
    /// ### Arguments
    ///
    /// * `package` - Built Move package
    fn generate_module_payload(package: &BuiltPackage) -> TransactionPayload {
        // extract package data
        let code = package.extract_code();
        let metadata = package
            .extract_metadata()
            .expect("extracting package metadata must succeed");

        // publish package similar to create_publish_package in harness.rs
        code_publish_package_txn(
            bcs::to_bytes(&metadata).expect("PackageMetadata has BCS"),
            code,
        )
    }

    /// Generate a TransactionPayload for scripts
    ///
    /// ### Arguments
    ///
    /// * `package` - Built Move package
    fn generate_script_payload_jwk(package: &BuiltPackage) -> TransactionPayload {
        // extract package data
        let code = package.extract_script_code().into_iter().next().unwrap();
        let ty_args = vec![];
        let args = vec![];

        TransactionPayload::Script(Script::new(code, ty_args, args))
    }

    /// Same as `publish_package` but as an entry function which can be called as a transaction. Because
    /// of current restrictions for txn parameters, the metadata needs to be passed in serialized form.
    pub fn code_publish_package_txn(
        metadata_serialized: Vec<u8>,
        code: Vec<Vec<u8>>,
    ) -> TransactionPayload {
        TransactionPayload::EntryFunction(EntryFunction::new(
            ModuleId::new(
                AccountAddress::new([
                    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                    0, 0, 0, 0, 0, 1,
                ]),
                ident_str!("code").to_owned(),
            ),
            ident_str!("publish_package_txn").to_owned(),
            vec![],
            vec![
                bcs::to_bytes(&metadata_serialized).unwrap(),
                bcs::to_bytes(&code).unwrap(),
            ],
        ))
    }
}
