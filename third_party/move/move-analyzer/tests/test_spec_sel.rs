// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

#[cfg(test)]
mod tests {

    use aptos_move_analyzer::{
        context::*,
        move_generate_spec_sel::{on_generate_spec_sel, Resp},
        multiproject::MultiProject,
        utils::{discover_manifest_and_kind, path_concat},
    };
    use lsp_server::*;
    use serde_json::json;
    use std::path::PathBuf;
    // use itertools::Itertools;

    fn prepare_project(context: &mut Context, fpath: PathBuf) {
        log::info!("fpath = {:?}", fpath.to_str());
        let (mani, _) = match discover_manifest_and_kind(&fpath) {
            Some(x) => x,
            None => {
                log::error!("not move project.");
                return;
            },
        };
        match context.projects.get_project(&fpath) {
            Some(_) => {
                if let Ok(_x) = std::fs::read_to_string(fpath.as_path()) {
                    // update_defs_on_changed(context, fpath.clone(), x);
                };
                return;
            },
            None => {
                eprintln!("project '{:?}' not found try load.", fpath.as_path());
            },
        };
        let p = match context.projects.load_projects(&context.connection, &mani) {
            anyhow::Result::Ok(x) => x,
            anyhow::Result::Err(e) => {
                log::error!("load project failed,err:{:?}", e);
                return;
            },
        };
        context.projects.insert_project(p);
    }

    /// Generate Spec for Selected function with assert!()
    #[test]
    fn test_generate_spec_request_001() {
        let (connection, _) = Connection::stdio();
        let mut context = Context {
            projects: MultiProject::new(),
            connection,
            diag_version: FileDiags::new(),
        };

        let fpath = path_concat(
            std::env::current_dir().unwrap().as_path(),
            PathBuf::from("tests/v1-core/LPResourceAccount/sources/resourceAccount.move").as_path(),
        );
        prepare_project(&mut context, fpath.clone());

        let _ = match context.projects.get_project(&fpath) {
            Some(x) => x,
            None => {
                log::error!("project '{:?}' not found.", fpath.as_path());
                return;
            },
        };

        let params_json = json!({
            "col": 69,
            "line": 10,
            "fpath": fpath.to_string_lossy().to_string()
        });
        let request = Request {
            id: "generate_spec_request_001".to_string().into(),
            method: String::from("move/generate/spec/sel"),
            params: params_json,
        };

        let actual_r = on_generate_spec_sel(&mut context, &request);
        let ex = Some(Resp {
            line: 11,
            col: 4,
            content: String::from("    spec CapabilityStorage{\n    }\n"),
        });
        let expect_r = Response::new_ok(
            "generate_spec_request_001".to_string().into(),
            serde_json::to_value(ex).unwrap(),
        );
        // // std::thread::sleep(Duration::new(1, 0));
        // log::info!("\n------------------------------\n");
        eprintln!("actual_r = {:?}", actual_r.result);
        eprintln!("\n\n\n");
        // log::trace!("expect_r = {:?}", expect_r);
        log::info!("\n------------------------------\n");
        assert_eq!(actual_r.result, expect_r.result);
    }

    /// Generate Spec for Operators which may Overflow in a Function.
    #[test]
    fn test_generate_spec_request_002() {
        let (connection, _) = Connection::stdio();
        let mut context = Context {
            projects: MultiProject::new(),
            connection,
            diag_version: FileDiags::new(),
        };

        let fpath = path_concat(
            std::env::current_dir().unwrap().as_path(),
            PathBuf::from("tests/symbols/sources/SpecTest.move").as_path(),
        );
        prepare_project(&mut context, fpath.clone());

        let _ = match context.projects.get_project(&fpath) {
            Some(x) => x,
            None => {
                log::error!("project '{:?}' not found.", fpath.as_path());
                return;
            },
        };

        let params_json = json!({
            "col": 5,
            "line": 14,
            "fpath": fpath.to_string_lossy().to_string()
        });
        let request = Request {
            id: "generate_spec_request_002".to_string().into(),
            method: String::from("move/generate/spec/sel"),
            params: params_json,
        };

        let actual_r = on_generate_spec_sel(&mut context, &request);
        let ex = Some(
            Resp {
                line: 15,
                col: 5,
                content: String::from("    spec test_may_overflow(var_u64: u64, var_u128: u128, var_u256: u256): u64{\n        let var_local_u64 = var_u64 + 1;\n        aborts_if var_u64 + 1 > MAX_U64;\n        let var_local_u128 = var_u128 * 2;\n        aborts_if var_u128 * 2 > MAX_U128;\n        let var_local_u256 = var_u256 << 3;\n        aborts_if var_u256 << 3 > MAX_U256;\n        aborts_if (var_local_u64 as u128) + var_local_u128 > MAX_U128;\n        aborts_if (((var_local_u64 as u128) + var_local_u128) as u256) * var_local_u256 > MAX_U256;\n        aborts_if ((((var_local_u64 as u128) + var_local_u128) as u256) * var_local_u256) << 3 > MAX_U256;\n    }\n")
            }
        );
        let expect_r = Response::new_ok(
            "generate_spec_request_002".to_string().into(),
            serde_json::to_value(ex).unwrap(),
        );
        // // std::thread::sleep(Duration::new(1, 0));
        // log::info!("\n------------------------------\n");
        eprintln!("actual_r = {:?}", actual_r.result);
        eprintln!("\n\n\n");
        // log::trace!("expect_r = {:?}", expect_r);
        log::info!("\n------------------------------\n");
        assert_eq!(actual_r.result, expect_r.result);
    }

    /// Generate Spec for Operators which may Underflow in a Function.
    #[test]
    fn test_generate_spec_request_003() {
        let (connection, _) = Connection::stdio();
        let mut context = Context {
            projects: MultiProject::new(),
            connection,
            diag_version: FileDiags::new(),
        };

        let fpath = path_concat(
            std::env::current_dir().unwrap().as_path(),
            PathBuf::from("tests/symbols/sources/SpecTest.move").as_path(),
        );
        prepare_project(&mut context, fpath.clone());

        let _ = match context.projects.get_project(&fpath) {
            Some(x) => x,
            None => {
                log::error!("project '{:?}' not found.", fpath.as_path());
                return;
            },
        };

        let params_json = json!({
            "col": 5,
            "line": 24,
            "fpath": fpath.to_string_lossy().to_string()
        });
        let request = Request {
            id: "generate_spec_request_003".to_string().into(),
            method: String::from("move/generate/spec/sel"),
            params: params_json,
        };

        let actual_r = on_generate_spec_sel(&mut context, &request);
        let ex = Some(
            Resp {
                line: 25,
                col: 5,
                content: String::from("    spec test_may_underflow(var_u64: u64, var_u128: u128, var_u256: u256): u64{\n        aborts_if var_u64 - 1 < 0;\n        let var_local_u128 = (var_u128 * 2) - 1000;\n        aborts_if var_u128 * 2 > MAX_U128;\n        aborts_if (var_u128 * 2) - 1000 < 0;\n        aborts_if var_local_u128 <= 0 with 0;\n    }\n")
            }
        );
        let expect_r = Response::new_ok(
            "generate_spec_request_003".to_string().into(),
            serde_json::to_value(ex).unwrap(),
        );
        // // std::thread::sleep(Duration::new(1, 0));
        // log::info!("\n------------------------------\n");
        eprintln!("actual_r = {:?}", actual_r.result);
        eprintln!("\n\n\n");
        // log::trace!("expect_r = {:?}", expect_r);
        log::info!("\n------------------------------\n");
        assert_eq!(actual_r.result, expect_r.result);
    }

    /// Generate Spec for Operators which may Div Zero in a Function.
    #[test]
    fn test_generate_spec_request_004() {
        let (connection, _) = Connection::stdio();
        let mut context = Context {
            projects: MultiProject::new(),
            connection,
            diag_version: FileDiags::new(),
        };

        let fpath = path_concat(
            std::env::current_dir().unwrap().as_path(),
            PathBuf::from("tests/symbols/sources/SpecTest.move").as_path(),
        );
        prepare_project(&mut context, fpath.clone());

        let _ = match context.projects.get_project(&fpath) {
            Some(x) => x,
            None => {
                log::error!("project '{:?}' not found.", fpath.as_path());
                return;
            },
        };

        let params_json = json!({
            "col": 5,
            "line": 33,
            "fpath": fpath.to_string_lossy().to_string()
        });
        let request = Request {
            id: "generate_spec_request_004".to_string().into(),
            method: String::from("move/generate/spec/sel"),
            params: params_json,
        };

        let actual_r = on_generate_spec_sel(&mut context, &request);
        let ex = Some(
             Resp {
                 line: 34,
                 col: 5,
                 content: String::from("    spec test_may_div_zero(var_u64: u64, var_u128: u128, var_u256: u256): u64{\n        aborts_if 100 + var_u64 > MAX_U64;\n        aborts_if var_u64 == 0;\n        aborts_if var_u256 == 0;\n        aborts_if ((var_u128 as u256) / var_u256) == 0;\n    }\n")
             }
         );
        let expect_r = Response::new_ok(
            "generate_spec_request_004".to_string().into(),
            serde_json::to_value(ex).unwrap(),
        );
        // // std::thread::sleep(Duration::new(1, 0));
        // log::info!("\n------------------------------\n");
        eprintln!("actual_r = {:?}", actual_r.result);
        eprintln!("\n\n\n");
        // log::trace!("expect_r = {:?}", expect_r);
        log::info!("\n------------------------------\n");
        assert_eq!(actual_r.result, expect_r.result);
    }
}
