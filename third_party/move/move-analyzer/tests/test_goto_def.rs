// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

#[cfg(test)]
mod tests {

    use aptos_move_analyzer::{
        context::*,
        goto_definition::on_go_to_def_request,
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

    /// Testing whether local variables in a test function
    /// can goto definition in this function
    #[test]
    fn test_on_go_to_def_request_001() {
        let (connection, _) = Connection::stdio();
        let mut context = Context {
            projects: MultiProject::new(),
            connection,
            diag_version: FileDiags::new(),
        };

        let fpath = path_concat(
            std::env::current_dir().unwrap().as_path(),
            PathBuf::from("tests/v1-core/Swap/sources/swap.move").as_path(),
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
            "position": {
                "line": 188,
                "character": 10

            },
            "textDocument": {
                "uri": "file://".to_string() + fpath.to_str().unwrap()
            },
        });
        let request = Request {
            id: "go_to_def_request_001".to_string().into(),
            method: String::from("textDocument/definition"),
            params: params_json,
        };

        let actual_r = on_go_to_def_request(&context, &request);
        let expect_r = Response::new_ok(
            "go_to_def_request_001".to_string().into(),
            json!([{
                "range":{
                    "end":{
                        "character":12,
                        "line":187
                    },
                    "start":{
                        "character":12,
                        "line":187
                    }
                },
                "uri": "file://".to_string() + fpath.to_str().unwrap()

            }]),
        );
        // std::thread::sleep(Duration::new(1, 0));
        log::trace!("\n------------------------------\n");
        log::trace!("actual_r = {:?}", actual_r);
        log::trace!("\n");
        log::trace!("expect_r = {:?}", expect_r);
        log::trace!("\n------------------------------\n");
        assert_eq!(actual_r.result, expect_r.result);
    }

    /// Test whether references to structs in the same file can get definition.
    #[test]
    fn test_on_go_to_def_request_002() {
        let (connection, _) = Connection::stdio();
        let mut context = Context {
            projects: MultiProject::new(),
            connection,
            diag_version: FileDiags::new(),
        };

        let fpath = path_concat(
            std::env::current_dir().unwrap().as_path(),
            PathBuf::from("tests/v1-core/Swap/sources/swap.move").as_path(),
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
            "position": {
                "line": 160,
                "character": 39
            },
            "textDocument": {
                "uri": "file://".to_string() + fpath.to_str().unwrap()
            },
        });
        let request = Request {
            id: "go_to_def_request_001".to_string().into(),
            method: String::from("textDocument/definition"),
            params: params_json,
        };

        let actual_r = on_go_to_def_request(&context, &request);
        let expect_r = Response::new_ok(
            "go_to_def_request_001".to_string().into(),
            json!([{
                "range":{
                    "end":{
                        "character":29,
                        "line":48
                    },
                    "start":{
                        "character":4,
                        "line":48
                    }
                },
                "uri": "file://".to_string() + fpath.to_str().unwrap()
            }]),
        );
        // std::thread::sleep(Duration::new(1, 0));
        log::trace!("\n------------------------------\n");
        log::trace!("actual_r = {:?}", actual_r);
        log::trace!("\n");
        log::trace!("expect_r = {:?}", expect_r);
        log::trace!("\n------------------------------\n");
        assert_eq!(actual_r.result, expect_r.result);
    }

    /// Test function call can get definition
    #[test]
    fn test_on_go_to_def_request_003() {
        let (connection, _) = Connection::stdio();
        let mut context = Context {
            projects: MultiProject::new(),
            connection,
            diag_version: FileDiags::new(),
        };

        let fpath = path_concat(
            std::env::current_dir().unwrap().as_path(),
            PathBuf::from("tests/v1-core/Swap/sources/swap.move").as_path(),
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
            "position": {
                "line": 543,
                "character": 43
            },
            "textDocument": {
                "uri": "file://".to_string() + fpath.to_str().unwrap()
            },
        });
        let request = Request {
            id: "go_to_def_request_001".to_string().into(),
            method: String::from("textDocument/definition"),
            params: params_json,
        };

        let actual_r = on_go_to_def_request(&context, &request);
        let expect_r = Response::new_ok(
            "go_to_def_request_001".to_string().into(),
            json!([{
                "range":{
                    "start":{
                        "line":228,
                        "character":4,
                    },
                    "end":{
                        "line":228,
                        "character":42,
                    },
                },
                "uri": "file://".to_string() + fpath.to_str().unwrap()
            }]),
        );
        // std::thread::sleep(Duration::new(1, 0));
        log::trace!("\n------------------------------\n");
        log::trace!("actual_r = {:?}", actual_r);
        log::trace!("\n");
        log::trace!("expect_r = {:?}", expect_r);
        log::trace!("\n------------------------------\n");
        assert_eq!(actual_r.result, expect_r.result);
    }

    /// Test CONST var can get definition
    #[test]
    fn test_on_go_to_def_request_004() {
        let (connection, _) = Connection::stdio();
        let mut context = Context {
            projects: MultiProject::new(),
            connection,
            diag_version: FileDiags::new(),
        };

        let fpath = path_concat(
            std::env::current_dir().unwrap().as_path(),
            PathBuf::from("tests/v1-core/Swap/sources/swap.move").as_path(),
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
            "position": {
                "line": 274,
                "character": 86
            },
            "textDocument": {
                "uri": "file://".to_string() + fpath.to_str().unwrap()
            },
        });
        let request = Request {
            id: "go_to_def_request_001".to_string().into(),
            method: String::from("textDocument/definition"),
            params: params_json,
        };

        let actual_r = on_go_to_def_request(&context, &request);
        let expect_r = Response::new_ok(
            "go_to_def_request_001".to_string().into(),
            json!([{
                "range":{
                    "start":{
                        "line":140,
                        "character":4,
                    },
                    "end":{
                        "line":140,
                        "character":4,
                    },
                },
                "uri": "file://".to_string() + fpath.to_str().unwrap()
            }]),
        );
        // std::thread::sleep(Duration::new(1, 0));
        log::trace!("\n------------------------------\n");
        log::trace!("actual_r = {:?}", actual_r);
        log::trace!("\n");
        log::trace!("expect_r = {:?}", expect_r);
        log::trace!("\n------------------------------\n");
        assert_eq!(actual_r.result, expect_r.result);
    }

    /// Test cross-file function definition within the same project
    #[test]
    fn test_on_go_to_def_request_005() {
        let (connection, _) = Connection::stdio();
        let mut context = Context {
            projects: MultiProject::new(),
            connection,
            diag_version: FileDiags::new(),
        };

        let fpath = path_concat(
            std::env::current_dir().unwrap().as_path(),
            PathBuf::from("tests/v1-core/Swap/sources/swap.move").as_path(),
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
            "position": {
                "line": 199,
                "character": 56
            },
            "textDocument": {
                "uri": "file://".to_string() + fpath.to_str().unwrap()
            },
        });
        let request = Request {
            id: "go_to_def_request_001".to_string().into(),
            method: String::from("textDocument/definition"),
            params: params_json,
        };

        let actual_r = on_go_to_def_request(&context, &request);
        let expect_r = Response::new_ok(
            "go_to_def_request_001".to_string().into(),
            json!([{
                "range":{
                    "start":{
                        "line":35,
                        "character":4,
                    },
                    "end":{
                        "line":35,
                        "character":42,
                    },
                },
                "uri":
                    "file://".to_string()
                    + path_concat(
                        std::env::current_dir().unwrap().as_path(),
                        PathBuf::from("tests/v1-core/Swap/sources/swap_library.move").as_path(),
                    ).to_str().unwrap()
            }]),
        );
        // std::thread::sleep(Duration::new(1, 0));
        log::trace!("\n------------------------------\n");
        log::trace!("actual_r = {:?}", actual_r);
        log::trace!("\n");
        log::trace!("expect_r = {:?}", expect_r);
        log::trace!("\n------------------------------\n");
        assert_eq!(actual_r.result, expect_r.result);
    }

    /// Test goto definition for use decl
    #[test]
    fn test_on_go_to_def_request_006() {
        let (connection, _) = Connection::stdio();
        let mut context = Context {
            projects: MultiProject::new(),
            connection,
            diag_version: FileDiags::new(),
        };

        let fpath = path_concat(
            std::env::current_dir().unwrap().as_path(),
            PathBuf::from("tests/v1-core/Swap/sources/swap.move").as_path(),
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
            "position": {
                "line": 1,
                "character": 47
            },
            "textDocument": {
                "uri": "file://".to_string() + fpath.to_str().unwrap()
            },
        });
        let request = Request {
            id: "go_to_def_request_001".to_string().into(),
            method: String::from("textDocument/definition"),
            params: params_json,
        };

        let actual_r = on_go_to_def_request(&context, &request);
        let expect_r = Response::new_ok(
            "go_to_def_request_001".to_string().into(),
            json!([{
                "range":{
                    "start":{
                        "line":1,
                        "character":4,
                    },
                    "end":{
                        "line":1,
                        "character":4,
                    },
                },
                "uri":
                    "file://".to_string()
                    + path_concat(
                        std::env::current_dir().unwrap().as_path(),
                        PathBuf::from("tests/v1-core/LPCoin/sources/LPCoin.move").as_path(),
                    ).to_str().unwrap()
            }]),
        );
        assert_eq!(actual_r.result, expect_r.result);
    }
}
