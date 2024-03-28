// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

#[cfg(test)]
mod tests {
    use aptos_move_analyzer::{
        context::{Context, FileDiags},
        hover,
        multiproject::MultiProject,
        utils::*,
    };
    use lsp_server::{Connection, Request, Response};
    use serde_json::json;
    use std::{path::PathBuf, time::Duration};

    fn prepare_project(context: &mut Context, fpath: PathBuf) {
        eprintln!("<UT> -- prepare_project -- fpath = {:?}", fpath.to_str());
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

    // test: func call
    #[test]
    fn test_hover_001() {
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

        let params_json = json!({
            "context": {
                "includeDeclaration": true
            },
            "position": {
                "line": 199,
                "character": 53
            },
            "textDocument": {
                "uri": "file:///".to_string() + fpath.to_str().unwrap()
            },
        });
        let request = Request {
            id: "hover_request_001".to_string().into(),
            method: String::from("textDocument/hover"),
            params: params_json,
        };

        let expect_r = Response::new_ok(
            "hover_request_001".to_string().into(),
            json!({
                    "contents": "public fun get_amount_out(amount_in: u64,reserve_in: u64,reserve_out: u64,swap_fee: u64): u64"
                }
            ),
        );

        let actual_r = hover::on_hover_request(&context, &request);
        std::thread::sleep(Duration::new(1, 0));
        eprintln!("\n------------------------------\n");
        eprintln!("actual_r = {:?}", serde_json::to_string(&actual_r));
        eprintln!("\n------------------------------\n");
        assert_eq!(actual_r.result, expect_r.result);
    }

    // test: const
    #[test]
    fn test_hover_002() {
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

        let params_json = json!({
            "context": {
                "includeDeclaration": true
            },
            "position": {
                "line": 295,
                "character": 72
            },
            "textDocument": {
                "uri": "file:///".to_string() + fpath.to_str().unwrap()
            },
        });
        let request: Request = Request {
            id: "test_hover_002".to_string().into(),
            method: String::from("textDocument/hover"),
            params: params_json,
        };

        let expect_r = Response::new_ok(
            "test_hover_002".to_string().into(),
            json!({
                    "contents": "0x796900ebe1a1a54ff9e932f19c548f5c1af5c6e7d34965857ac2f7b1d1ab2cbf"
                }
            ),
        );

        let actual_r = hover::on_hover_request(&context, &request);
        std::thread::sleep(Duration::new(1, 0));
        eprintln!("\n------------------------------\n");
        eprintln!("actual_r = {:?}", serde_json::to_string(&actual_r));
        eprintln!("\n------------------------------\n");
        assert_eq!(actual_r.result, expect_r.result);
    }

    // test: var in if-else expr
    #[test]
    fn test_hover_003() {
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

        let params_json = json!({
            "context": {
                "includeDeclaration": true
            },
            "position": {
                "line": 301,
                "character": 19
            },
            "textDocument": {
                "uri": "file:///".to_string() + fpath.to_str().unwrap()
            },
        });
        let request: Request = Request {
            id: "test_hover_003".to_string().into(),
            method: String::from("textDocument/hover"),
            params: params_json,
        };

        let expect_r = Response::new_ok(
            "test_hover_003".to_string().into(),
            json!({
                    "contents": "u64"
                }
            ),
        );

        let actual_r = hover::on_hover_request(&context, &request);
        std::thread::sleep(Duration::new(1, 0));
        eprintln!("\n------------------------------\n");
        eprintln!("actual_r = {:?}", serde_json::to_string(&actual_r));
        eprintln!("\n------------------------------\n");
        assert_eq!(actual_r.result, expect_r.result);
    }

    // test: var in assert
    #[test]
    fn test_hover_004() {
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

        let params_json = json!({
            "context": {
                "includeDeclaration": true
            },
            "position": {
                "line": 307,
                "character": 47
            },
            "textDocument": {
                "uri": "file:///".to_string() + fpath.to_str().unwrap()
            },
        });
        let request: Request = Request {
            id: "test_hover_004".to_string().into(),
            method: String::from("textDocument/hover"),
            params: params_json,
        };

        let expect_r = Response::new_ok(
            "test_hover_004".to_string().into(),
            json!({
                    "contents": "u64"
                }
            ),
        );

        let actual_r = hover::on_hover_request(&context, &request);
        std::thread::sleep(Duration::new(1, 0));
        eprintln!("\n------------------------------\n");
        eprintln!("actual_r = {:?}", serde_json::to_string(&actual_r));
        eprintln!("\n------------------------------\n");
        assert_eq!(actual_r.result, expect_r.result);
    }

    // test: var in tuple
    #[test]
    fn test_hover_005() {
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

        let params_json = json!({
            "context": {
                "includeDeclaration": true
            },
            "position": {
                "line": 308,
                "character": 22
            },
            "textDocument": {
                "uri": "file:///".to_string() + fpath.to_str().unwrap()
            },
        });
        let request: Request = Request {
            id: "test_hover_005".to_string().into(),
            method: String::from("textDocument/hover"),
            params: params_json,
        };

        let expect_r = Response::new_ok(
            "test_hover_005".to_string().into(),
            json!({
                    "contents": "u64"
                }
            ),
        );

        let actual_r = hover::on_hover_request(&context, &request);
        std::thread::sleep(Duration::new(1, 0));
        eprintln!("\n------------------------------\n");
        eprintln!("actual_r = {:?}", serde_json::to_string(&actual_r));
        eprintln!("\n------------------------------\n");
        assert_eq!(actual_r.result, expect_r.result);
    }

    // test: struct
    #[test]
    fn test_hover_006() {
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

        let params_json = json!({
            "context": {
                "includeDeclaration": true
            },
            "position": {
                "line": 398,
                "character": 100
            },
            "textDocument": {
                "uri": "file:///".to_string() + fpath.to_str().unwrap()
            },
        });
        let request: Request = Request {
            id: "test_hover_006".to_string().into(),
            method: String::from("textDocument/hover"),
            params: params_json,
        };

        let expect_r = Response::new_ok(
            "test_hover_006".to_string().into(),
            json!({
                    "contents": "account::SignerCapability"
                }
            ),
        );

        let actual_r = hover::on_hover_request(&context, &request);
        std::thread::sleep(Duration::new(1, 0));
        eprintln!("\n------------------------------\n");
        eprintln!("actual_r = {:?}", serde_json::to_string(&actual_r));
        eprintln!("\n------------------------------\n");
        assert_eq!(actual_r.result, expect_r.result);
    }
}
