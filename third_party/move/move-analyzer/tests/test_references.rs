// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

#[cfg(test)]
mod tests {
    use aptos_move_analyzer::{
        context::{Context, FileDiags},
        multiproject::MultiProject,
        references,
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

    /// test: comparator::is_smaller_than from aptos-move/framework/aptos-stdlib
    #[test]
    fn test_on_references_request_001() {
        let (connection, _) = Connection::stdio();
        let mut context = Context {
            projects: MultiProject::new(),
            connection,
            diag_version: FileDiags::new(),
        };

        let fpath = path_concat(
            std::env::current_dir().unwrap().as_path(),
            PathBuf::from("tests/v1-core/Swap/sources/swap_library.move").as_path(),
        );
        prepare_project(&mut context, fpath.clone());

        let params_json = json!({
            "context": {
                "includeDeclaration": true
            },
            "position": {
                "line": 131,
                "character": 29
            },
            "textDocument": {
                "uri": "file:///".to_string() + fpath.to_str().unwrap()
            },
        });
        let request = Request {
            id: "references_request_001".to_string().into(),
            method: String::from("textDocument/references"),
            params: params_json,
        };

        let expect_r = Response::new_ok(
            "references_request_001".to_string().into(),
            json!([
                {
                    "range": {
                            "end": {
                                "character": 35,
                                "line": 131
                            },
                            "start": {
                                "character": 20,
                                "line": 131
                            }
                    },

                    "uri": ("file://".to_string() + path_concat(
                                    std::env::current_dir().unwrap().as_path(),
                                    PathBuf::from("tests/v1-core/Swap/sources/swap_library.move").as_path()).to_str().unwrap()
                            ).replace('\\', "/")
                },
                {
                    "range": {
                            "end": {
                                "character": 43,
                                "line": 123
                            },
                            "start": {
                                "character": 28,
                                "line": 123
                            }
                    },
                    "uri": ("file://".to_string() + path_concat(
                        home::home_dir().unwrap().as_path(),
                        PathBuf::from(".move/https___github_com_aptos-labs_aptos-core_git_83641d3f083762f31700060e5e95fbb1cbc28bf0/aptos-move/framework/aptos-stdlib/sources/simple_map.move")
                            .as_path()).to_str().unwrap()
                    ).replace('\\', "/")
                }
            ]),
        );

        let actual_r = references::on_references_request(&context, &request);
        std::thread::sleep(Duration::new(1, 0));
        eprintln!("\n\n------------------------------");
        eprintln!("actual_r = {:?}", serde_json::to_string(&actual_r));
        eprintln!("------------------------------\n");
        if let Some(serde_json::Value::Array(actual_r_result)) = actual_r.result {
            assert_eq!(actual_r_result.len(), 2);
            for actual_r_value in actual_r_result {
                let mut found_same_item = false;
                if let Some(serde_json::Value::Array(expect_r_result)) = expect_r.result.clone() {
                    for expect_r_value in expect_r_result {
                        if actual_r_value.eq(&expect_r_value) {
                            eprintln!(
                                "actual_r_value = {:?}",
                                serde_json::to_string(&actual_r_value)
                            );
                            eprintln!(
                                "expect_r_value = {:?}",
                                serde_json::to_string(&expect_r_value)
                            );
                            found_same_item = true;
                            break;
                        }
                    }
                }
                assert!(found_same_item,);
            }
        }
    }

    /// test: get_reserves_size from tests/v1-core/Swap/sources/swap.move
    #[test]
    fn test_on_references_request_002() {
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
                "line": 186,
                "character": 57
            },
            "textDocument": {
                "uri": "file:///".to_string() + fpath.to_str().unwrap()
            },
        });
        let request = Request {
            id: "references_request_002".to_string().into(),
            method: String::from("textDocument/references"),
            params: params_json,
        };

        let expect_r = Response::new_ok(
            "references_request_002".to_string().into(),
            json!([
                {
                    "range": {
                            "end": {
                                "character": 57,
                                "line": 222
                            },
                            "start": {
                                "character": 40,
                                "line": 222
                            }
                    },

                    "uri": ("file://".to_string() + path_concat(
                                    std::env::current_dir().unwrap().as_path(),
                                    PathBuf::from("tests/v1-core/Swap/sources/swap.move").as_path()).to_str().unwrap()
                            ).replace('\\', "/")
                },
                {
                    "range": {
                            "end": {
                                "character": 57,
                                "line": 232
                            },
                            "start": {
                                "character": 40,
                                "line": 232
                            }
                    },

                    "uri": ("file://".to_string() + path_concat(
                                    std::env::current_dir().unwrap().as_path(),
                                    PathBuf::from("tests/v1-core/Swap/sources/swap.move").as_path()).to_str().unwrap()
                            ).replace('\\', "/")
                },
                {
                    "range": {
                            "end": {
                                "character": 57,
                                "line": 244
                            },
                            "start": {
                                "character": 40,
                                "line": 244
                            }
                    },

                    "uri": ("file://".to_string() + path_concat(
                                    std::env::current_dir().unwrap().as_path(),
                                    PathBuf::from("tests/v1-core/Swap/sources/swap.move").as_path()).to_str().unwrap()
                            ).replace('\\', "/")
                },
                {
                    "range": {
                            "end": {
                                "character": 57,
                                "line": 186
                            },
                            "start": {
                                "character": 40,
                                "line": 186
                            }
                    },

                    "uri": ("file://".to_string() + path_concat(
                                    std::env::current_dir().unwrap().as_path(),
                                    PathBuf::from("tests/v1-core/Swap/sources/swap.move").as_path()).to_str().unwrap()
                            ).replace('\\', "/")
                },
                {
                    "range": {
                            "end": {
                                "character": 57,
                                "line": 196
                            },
                            "start": {
                                "character": 40,
                                "line": 196
                            }
                    },

                    "uri": ("file://".to_string() + path_concat(
                                    std::env::current_dir().unwrap().as_path(),
                                    PathBuf::from("tests/v1-core/Swap/sources/swap.move").as_path()).to_str().unwrap()
                            ).replace('\\', "/")
                },
                {
                    "range": {
                            "end": {
                                "character": 57,
                                "line": 208
                            },
                            "start": {
                                "character": 40,
                                "line": 208
                            }
                    },

                    "uri": ("file://".to_string() + path_concat(
                                    std::env::current_dir().unwrap().as_path(),
                                    PathBuf::from("tests/v1-core/Swap/sources/swap.move").as_path()).to_str().unwrap()
                            ).replace('\\', "/")
                },
                {
                    "range": {
                            "end": {
                                "character": 57,
                                "line": 753
                            },
                            "start": {
                                "character": 40,
                                "line": 753
                            }
                    },

                    "uri": ("file://".to_string() + path_concat(
                                    std::env::current_dir().unwrap().as_path(),
                                    PathBuf::from("tests/v1-core/Swap/sources/swap.move").as_path()).to_str().unwrap()
                            ).replace('\\', "/")
                },
            ]),
        );

        let actual_r = references::on_references_request(&context, &request);
        std::thread::sleep(Duration::new(1, 0));
        eprintln!("\n\n------------------------------");
        eprintln!("actual_r = {:?}", serde_json::to_string(&actual_r));
        eprintln!("------------------------------\n");
        if let Some(serde_json::Value::Array(actual_r_result)) = actual_r.result {
            assert_eq!(actual_r_result.len(), 7);
            for actual_r_value in actual_r_result {
                let mut found_same_item = false;
                if let Some(serde_json::Value::Array(expect_r_result)) = expect_r.result.clone() {
                    for expect_r_value in expect_r_result {
                        if actual_r_value.eq(&expect_r_value) {
                            eprintln!(
                                "actual_r_value = {:?}",
                                serde_json::to_string(&actual_r_value)
                            );
                            eprintln!(
                                "expect_r_value = {:?}",
                                serde_json::to_string(&expect_r_value)
                            );
                            found_same_item = true;
                            break;
                        }
                    }
                }
                assert!(found_same_item, "{}", true);
            }
        }
    }
}
