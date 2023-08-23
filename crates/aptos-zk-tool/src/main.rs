// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use anyhow::Result;
use clap::{Parser, Subcommand};
use std::collections::BTreeMap;
use std::{path::PathBuf, process::Command};
use ark_bls12_381::Bls12_381;
use ark_circom::CircomBuilder;
use ark_circom::CircomConfig;
use ark_groth16::{Groth16, ProvingKey};
use ark_std::rand::thread_rng;
use ark_serialize::{CanonicalSerialize, CanonicalDeserialize};
use ark_snark::SNARK;

#[derive(Parser)]
pub struct Argument {
    #[clap(subcommand)]
    cmd: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    GenerateVerifier {
        #[clap(short, long)]
        circuit_path: PathBuf,
    },
    Prove {
        #[clap(short, long)]
        circuit_path: PathBuf,

        #[clap(long)]
        input: String,
    }
}

fn main() -> Result<()> {
    let args = Argument::parse();

    match args.cmd {
        Commands::GenerateVerifier {
            circuit_path,
        } => {
            let mut output_path = circuit_path.clone();
            output_path.pop();
            output_path.push("target");

            std::fs::create_dir_all(&output_path)?;

            // Compile circuit to binary
            Command::new("circom")
                .args([
                    circuit_path.as_os_str().to_str().unwrap(),
                    "--r1cs",
                    "--wasm",
                    "-o",
                    output_path.as_os_str().to_str().unwrap(),
                ])
                .output()?;

            let base_name = circuit_path.file_stem().unwrap().to_str().unwrap();
            let mut wasm_path = output_path.clone();
            wasm_path.push(format!("{}_js/{}.wasm", base_name, base_name));
            let mut r1cs_path = output_path.clone();
            r1cs_path.push(format!("{}.r1cs", base_name));

            let cfg = CircomConfig::<Bls12_381>::new(wasm_path, r1cs_path).unwrap();
            let builder = CircomBuilder::new(cfg);
            let circom = builder.setup();

            let mut move_module_path = output_path.clone();
            move_module_path.push(format!("{}.move", base_name));
            let mut rng = thread_rng();

            let params =
                Groth16::<Bls12_381>::generate_random_parameters_with_reduction(circom, &mut rng).unwrap();

            let mut proving_key = output_path.clone();
            proving_key.push("proving_key.key");

            let proving_key_file = std::fs::File::create(&proving_key)?;
            let mut bytes = vec![];
            params.serialize_uncompressed(&proving_key_file)?;
            params.serialize_uncompressed(&mut bytes)?;
            println!("{:?}", bytes);

            aptos_zk_tool::export_move_module(
                &params.vk,
                move_module_path,
                base_name.to_owned(),
            )
        },
        Commands::Prove { circuit_path, input } => {
            let mut output_path = circuit_path.clone();
            output_path.pop();
            output_path.push("target");

            let base_name = circuit_path.file_stem().unwrap().to_str().unwrap();
            let mut wasm_path = output_path.clone();
            wasm_path.push(format!("{}_js/{}.wasm", base_name, base_name));
            let mut r1cs_path = output_path.clone();
            r1cs_path.push(format!("{}.r1cs", base_name));

            let input = serde_json::from_str::<BTreeMap<String, u64>>(input.as_str())?;
            let cfg = CircomConfig::<Bls12_381>::new(wasm_path, r1cs_path).unwrap();
            let mut builder = CircomBuilder::new(cfg);

            for (k, v) in input {
                builder.push_input(k, v);
            }
            let circom = builder.build().unwrap();
            let inputs = circom.get_public_inputs().unwrap();

            let mut proving_key = output_path.clone();
            proving_key.push("proving_key.key");

            let proving_key_file = std::fs::File::open(&proving_key)?;

            let proving_key = ProvingKey::<Bls12_381>::deserialize_uncompressed(&proving_key_file).unwrap();

            let mut rng = thread_rng();
            let proof = Groth16::<Bls12_381>::prove(&proving_key, circom, &mut rng).unwrap();

            macro_rules! key_to_string {
                ($key: expr) => {
                    {
                        let mut writer = vec![];
                        $key.serialize_uncompressed(&mut writer).unwrap();
                        hex::encode(&writer)
                    }
                };
            }

            let mut public_input_str = "".to_string();
            for (idx, input) in inputs.iter().enumerate() {
                if idx == inputs.len() - 1 {
                    public_input_str += key_to_string!(input).as_str();
                } else {
                    public_input_str += key_to_string!(input).as_str();
                    public_input_str += ", ";
                }
            }

            println!("proof: ");
            println!("(public_inputs) string:[\"{}\"]", public_input_str);
            println!("(proof_a) string:{}", key_to_string!(proof.a));
            println!("(proof_b) string:{}", key_to_string!(proof.b));
            println!("(proof_c) string:{}", key_to_string!(proof.c));
            Ok(())
        }
    }
}
