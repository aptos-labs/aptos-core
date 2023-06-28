// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

#[macro_use]
extern crate serde_derive;
extern crate mustache;

use clap::{command, Arg};
use std::{collections::HashMap, error::Error, fs::File, path::Path};

static TESTCASE_TEMPLATE: &str = r##"// {{result}}
module 0x101::Test1 {
  public fun test_{{name}}({{parameters}}): {{return_type}} {
    {{body}}
  }
}

script {
  fun main() {
    assert!(0x101::Test1::test_{{name}}({{valid_arguments}}) == {{expected}}, 10);  // Ok: source fits in dest.

    0x101::Test1::test_{{name}}({{wrong_arguments}});  // Abort: source too big.
  }
}
"##;

static EQ_TESTCASE_TEMPLATE: &str = r##"//
module 0x101::Test1 {
  public fun test_{{name}}({{parameters}}): {{return_type}} {
    {{body}}
  }
}

script {
  fun main() {
    {{expr}}
  }
}
"##;

type MyResult<T> = Result<T, Box<dyn Error>>;

#[derive(Debug)]
pub enum Generators {
    All,
    Cast,
    Eq,
}

#[derive(Debug)]
pub struct Config {
    pub generators: Vec<Generators>,
    pub out_dir: Option<String>,
}

#[derive(Serialize)]
struct Instance {
    name: String,
    parameters: String,
    return_type: String,
    body: String,
    result: String,
    valid_arguments: String,
    wrong_arguments: String,
    expected: String,
}

#[derive(Serialize)]
struct EqInstance {
    name: String,
    parameters: String,
    return_type: String,
    body: String,
    expr: String,
}

macro_rules! make_test_filename {
    (
        $filename:expr,
        $config:expr
    ) => {
        if let Some(path) = &$config.out_dir {
            Path::new(&path).join(&$filename)
        } else {
            Path::new(&$filename).to_path_buf()
        }
    };
}

fn generate_cast_tests(config: &Config) {
    let template = mustache::compile_str(TESTCASE_TEMPLATE).unwrap();

    let valid_values = HashMap::from([
        ("u8", "255"),
        ("u16", "65535"),
        ("u32", "4294967295"),
        ("u64", "18446744073709551615"),
        ("u128", "21267647932558653966460912964485513215"),
    ]);
    let wrong_values = HashMap::from([
        ("u8", "256"),
        ("u16", "65536"),
        ("u32", "4294967296"),
        ("u64", "18446744073709551616"),
        ("u128", "21267647932558653966460912964485513216"),
    ]);
    for (inx, in_ty) in ["u8", "u16", "u32", "u64", "u128", "u256"]
        .iter()
        .enumerate()
    {
        for (oux, ou_ty) in ["u8", "u16", "u32", "u64", "u128"].iter().enumerate() {
            if inx <= oux {
                continue;
            }
            let filename = format!("cast-{in_ty}-to-{ou_ty}-rangechk-abort.move");
            let name = format!("cast{in_ty}_{ou_ty}");
            let parameters = format!("a: {in_ty}");
            let return_type = ou_ty.to_string();
            let body = format!("(a as {ou_ty})");
            let valid_arguments = format!("{}{}", valid_values.get(ou_ty).unwrap(), in_ty);
            let wrong_arguments = format!("{}{}", wrong_values.get(ou_ty).unwrap(), in_ty);
            let expected = format!("{}{}", valid_values.get(ou_ty).unwrap(), ou_ty);
            let result = "abort 4017".to_string();
            let instance = Instance {
                name,
                parameters,
                return_type,
                body,
                result,
                valid_arguments,
                wrong_arguments,
                expected,
            };
            let filename = make_test_filename!(filename, config);
            let mut file = File::create(filename).unwrap();
            template.render(&mut file, &instance).unwrap();
        }
    }
}

fn generate_eq_neq_tests(config: &Config) {
    let template = mustache::compile_str(EQ_TESTCASE_TEMPLATE).unwrap();

    let valid_values = HashMap::from([
        ("u8", 0xffu128),
        ("u16", 0xffffu128),
        ("u32", 0xffffffffu128),
        ("u64", 0xffffffffffffffffu128),
        ("u128", 0xfffffffffffffffffffffffffffffffu128),
        ("u256", 0xfffffffffffffffffffffffffffffffu128),
    ]);
    for in_ty in ["u8", "u16", "u32", "u64", "u128", "u256"].iter() {
        let filename = format!("eq-{in_ty}.move");
        let name = format!("eq_{in_ty}");
        let parameters = format!("a: {in_ty}, b: {in_ty}");
        let return_type = "bool".to_string();
        let body = "a == b".to_string();
        let equal_arguments = format!(
            "{}{}, {}{}",
            valid_values.get(in_ty).unwrap(),
            in_ty,
            valid_values.get(in_ty).unwrap(),
            in_ty,
        );
        let unequal_arguments = format!(
            "{}{}, {}{}",
            valid_values.get(in_ty).unwrap(),
            in_ty,
            valid_values.get(in_ty).unwrap() - 1,
            in_ty,
        );
        let expr = format!(
            r##"assert!(0x101::Test1::test_{name}({equal_arguments}), 10);
    assert!(!0x101::Test1::test_{name}({unequal_arguments}), 10);"##
        );
        let instance = EqInstance {
            name,
            parameters: parameters.clone(),
            return_type: return_type.clone(),
            body,
            expr,
        };
        let filename = make_test_filename!(filename, config);
        let mut file = File::create(filename).unwrap();
        template.render(&mut file, &instance).unwrap();
        let filename = format!("neq-{in_ty}.move");
        let name = format!("neq_{in_ty}");
        let body = "a != b".to_string();
        let expr = format!(
            r##"assert!(0x101::Test1::test_{name}({unequal_arguments}), 10);
    assert!(!0x101::Test1::test_{name}({equal_arguments}), 10);"##
        );
        let instance = EqInstance {
            name,
            parameters,
            return_type,
            body,
            expr,
        };
        let filename = make_test_filename!(filename, config);
        let mut file = File::create(filename).unwrap();
        template.render(&mut file, &instance).unwrap();
    }
}

fn run(config: &Config) {
    if config
        .generators
        .iter()
        .any(|g| matches!(*g, Generators::All))
    {
        generate_cast_tests(config);
        generate_eq_neq_tests(config);
        std::process::exit(0);
    }
    for gen in &config.generators {
        match gen {
            Generators::Cast => generate_cast_tests(config),
            Generators::Eq => generate_eq_neq_tests(config),
            _ => {}
        }
    }
}

pub fn get_args() -> MyResult<Config> {
    let matches = command!()
        .about("Generate Move source code test files")
        .arg(
            Arg::new("generators")
                .value_name("GENERATOR")
                .help("Generators to run")
                .possible_values(["all", "cast", "equality"])
                .default_value("all")
                .multiple_values(true),
        )
        .arg(
            Arg::new("out_dir")
                .value_name("PATH")
                .short('o')
                .long("outdir")
                .help("Location of output files")
                .takes_value(true)
                .required(false),
        )
        .get_matches();
    let out_dir = if let Ok(out_dir) = matches.value_of_t::<String>("out_dir") {
        Some(out_dir)
    } else {
        None
    };
    Ok(Config {
        generators: matches
            .values_of_t::<String>("generators")
            .unwrap_or_default()
            .into_iter()
            .map(|v| match v.as_str() {
                "cast" => Generators::Cast,
                "equality" => Generators::Eq,
                "all" => Generators::All,
                _ => {
                    eprintln!("Unknown generator");
                    std::process::exit(1);
                }
            })
            .collect::<Vec<_>>(),
        out_dir,
    })
}

fn main() {
    match get_args() {
        Ok(config) => run(&config),
        Err(e) => {
            eprintln!("{}", e);
            std::process::exit(1);
        }
    }
}
