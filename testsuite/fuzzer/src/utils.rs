// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

#[allow(dead_code)]
#[allow(unused_variables)]
pub(crate) mod cli {
    use aptos_framework::{BuildOptions, BuiltPackage};
    use aptos_types::{
        account_address::AccountAddress,
        transaction::{EntryFunction, Script, TransactionPayloadWrapper},
    };
    use arbitrary::Arbitrary;
    use base64::{engine::general_purpose::STANDARD, Engine as _};
    use dearbitrary::Dearbitrary;
    use move_binary_format::file_format::{
        CompiledModule, CompiledScript, FunctionDefinitionIndex,
    };
    use move_core_types::{
        ident_str,
        identifier::Identifier,
        language_storage::{ModuleId, TypeTag},
        value::MoveValue,
    };
    use sha2::{Digest, Sha256};
    use std::{collections::HashMap, fs::File, io::Write, path::PathBuf};

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
    fn generate_module_payload(package: &BuiltPackage) -> TransactionPayloadWrapper {
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
    fn generate_script_payload_jwk(package: &BuiltPackage) -> TransactionPayloadWrapper {
        // extract package data
        let code = package.extract_script_code().into_iter().next().unwrap();
        let ty_args = vec![];
        let args = vec![];

        TransactionPayloadWrapper::Script(Script::new(code, ty_args, args))
    }

    /// Same as `publish_package` but as an entry function which can be called as a transaction. Because
    /// of current restrictions for txn parameters, the metadata needs to be passed in serialized form.
    pub fn code_publish_package_txn(
        metadata_serialized: Vec<u8>,
        code: Vec<Vec<u8>>,
    ) -> TransactionPayloadWrapper {
        TransactionPayloadWrapper::EntryFunction(EntryFunction::new(
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

    /// Data types for generate VM fuzzer corpora
    #[derive(Debug, Eq, PartialEq, arbitrary::Arbitrary, dearbitrary::Dearbitrary)]
    pub enum ExecVariant {
        Script {
            script: CompiledScript,
            type_args: Vec<TypeTag>,
            args: Vec<MoveValue>,
        },
        CallFunction {
            module: ModuleId,
            function: FunctionDefinitionIndex,
            type_args: Vec<TypeTag>,
            args: Vec<Vec<u8>>,
        },
    }

    #[derive(Debug, Arbitrary, Dearbitrary, Eq, PartialEq, Clone)]
    pub enum FundAmount {
        Zero,
        Poor,
        Rich,
    }

    #[derive(Debug, Arbitrary, Dearbitrary, Eq, PartialEq, Clone)]
    pub struct UserAccount {
        pub is_inited_and_funded: bool,
        pub fund: FundAmount,
    }

    #[derive(Debug, Arbitrary, Dearbitrary, Eq, PartialEq, Clone)]
    pub enum Authenticator {
        Ed25519 {
            sender: UserAccount,
        },
        MultiAgent {
            sender: UserAccount,
            secondary_signers: Vec<UserAccount>,
        },
        FeePayer {
            sender: UserAccount,
            secondary_signers: Vec<UserAccount>,
            fee_payer: UserAccount,
        },
    }
    #[derive(Debug, Eq, arbitrary::Arbitrary, dearbitrary::Dearbitrary, PartialEq)]
    pub struct RunnableState {
        pub dep_modules: Vec<CompiledModule>,
        pub exec_variant: ExecVariant,
        pub tx_auth_type: Authenticator,
    }

    /// Convert module to RunnableState data structure
    /// This is used to run the module in the VM
    pub(crate) fn generate_runnable_state(
        csv_path: &str,
        destination_path: &str,
    ) -> Result<(), String> {
        std::fs::create_dir_all(destination_path).map_err(|e| e.to_string())?;
        let runnable_states = to_runnablestate(&read_csv(csv_path));
        let mut cnt: usize = 0;
        println!("Number of runnable states: {} \n", runnable_states.len());
        for runnable_state in runnable_states {
            let d = runnable_state.0.dearbitrary_first();
            let bytes = d.finish();

            let u = arbitrary::Unstructured::new(&bytes);
            let runnable_state_generated =
                RunnableState::arbitrary_take_rest(u).map_err(|e| e.to_string())?;
            if runnable_state.0 != runnable_state_generated {
                println!(
                    "Failed <CompiledModule>: {:#?}",
                    runnable_state.0.dep_modules[0]
                );
                println!("Failed <Serialized>: {:#?} \n", runnable_state.1);
                cnt += 1;
            }
            let mut hasher = Sha256::new();
            hasher.update(&bytes);
            let hash = hasher.finalize();
            let filename = format!("{}/{}.bytes", destination_path, hex::encode(hash));
            let mut file = File::create(&filename).map_err(|e| e.to_string())?;
            file.write_all(&bytes).map_err(|e| e.to_string())?;
        }
        println!("Number of different states: {}", cnt);

        Ok(())
    }

    // function that read a CSV files and get the third element of each row and return a vector of bytes
    fn read_csv(path: &str) -> HashMap<ModuleId, (CompiledModule, String)> {
        let mut reader = csv::Reader::from_path(path).unwrap();

        let mut v = HashMap::new();

        for result in reader.records() {
            let record = result.unwrap();

            let module_name = record.get(0).unwrap();
            let address = record.get(1).unwrap();
            let bytecode = match STANDARD.decode(record.get(2).unwrap()) {
                Ok(bytecode) => bytecode,
                Err(err) => panic!(
                    "Error decoding {:?} B64 Decoding error: {:?}, Base64 string: {:?}",
                    address,
                    err,
                    record.get(2).unwrap()
                ),
            };
            /*
            let bytecode: Vec<u8> = match hex::decode(decoded_bytecode.clone()) {
                Ok(bytecode) => bytecode,
                Err(err) => panic!("Error decoding {:?}, Decoded Error: {:?}, bytecode: {:?}", address, err, decoded_bytecode),
            };
            */
            let serialized = record.get(2).unwrap();

            let account_address = match AccountAddress::from_hex_literal(address) {
                Ok(addr) => addr,
                Err(err) => {
                    eprintln!("Invalid address {}: {:?}", address, err);
                    continue;
                },
            };
            let identifier = match Identifier::new(module_name) {
                Ok(id) => id,
                Err(err) => {
                    eprintln!("Invalid module name {}: {:?}", module_name, err);
                    continue;
                },
            };
            let key = ModuleId::new(account_address, identifier);
            let compiled_module = match CompiledModule::deserialize(&bytecode) {
                Ok(module) => module,
                Err(err) => {
                    eprintln!("Error deserializing module: {:?}", err);
                    continue; // Skip to the next iteration of the loop
                },
            };
            v.insert(key, (compiled_module, serialized.to_string()));
        }
        v
    }

    fn to_runnablestate(
        map: &HashMap<ModuleId, (CompiledModule, String)>,
    ) -> Vec<(RunnableState, String)> {
        map.iter()
            .map(|(module_id, tuple)| {
                let runnable_state = RunnableState {
                    dep_modules: vec![tuple.0.to_owned()],
                    exec_variant: ExecVariant::CallFunction {
                        module: module_id.to_owned(),
                        function: FunctionDefinitionIndex::new(0),
                        type_args: vec![],
                        args: vec![],
                    },
                    tx_auth_type: Authenticator::Ed25519 {
                        sender: UserAccount {
                            is_inited_and_funded: true,
                            fund: FundAmount::Rich,
                        },
                    },
                };
                (runnable_state, tuple.1.to_owned())
            })
            .collect()
    }
}
