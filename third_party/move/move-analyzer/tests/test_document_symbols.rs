// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

#[cfg(test)]
mod tests {

    use aptos_move_analyzer::{
        context::*,
        multiproject::MultiProject,
        symbols::on_document_symbol_request,
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

    /// Document Symbol 1
    #[test]
    fn test_on_document_symbol_request_001() {
        let (connection, _) = Connection::stdio();
        let mut context = Context {
            projects: MultiProject::new(),
            connection,
            diag_version: FileDiags::new(),
        };

        let fpath = path_concat(
            std::env::current_dir().unwrap().as_path(),
            PathBuf::from("tests/v1-core/LPCoin/sources/LPCoin.move").as_path(),
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
            "textDocument": {
                "uri": "file://".to_string() + fpath.to_str().unwrap()
            },
        });
        let request = Request {
            id: "on_document_symbol_request_001".to_string().into(),
            method: String::from("textDocument/documentSymbol"),
            params: params_json,
        };

        let actual_r = on_document_symbol_request(&context, &request);
        let expect_r = Response::new_ok(
            "on_document_symbol_request_001".to_string().into(),
            json!(
                 [
                    {
                    "children": [
                        {
                        "children": [
                            {
                            "deprecated": false,
                            "kind": 8,
                            "name": "dummy_field",
                            "range": {
                                "end": {
                                "character": 17,
                                "line": 1
                                },
                                "start": {
                                "character": 11,
                                "line": 1
                                }
                            },
                            "selectionRange": {
                                "end": {
                                "character": 17,
                                "line": 1
                                },
                                "start": {
                                "character": 11,
                                "line": 1
                                }
                            },
                            "tags": [

                            ]
                            }
                        ],
                        "deprecated": false,
                        "kind": 14,
                        "name": "LPCoin",
                        "range": {
                            "end": {
                            "character": 58,
                            "line": 1
                            },
                            "start": {
                            "character": 4,
                            "line": 1
                            }
                        },
                        "selectionRange": {
                            "end": {
                            "character": 58,
                            "line": 1
                            },
                            "start": {
                            "character": 4,
                            "line": 1
                            }
                        },
                        "tags": [

                        ]
                        }
                    ],
                    "deprecated": false,
                    "detail": "LPCoinV1",
                    "kind": 2,
                    "name": "LPCoinV1",
                    "range": {
                        "end": {
                        "character": 1,
                        "line": 2
                        },
                        "start": {
                        "character": 0,
                        "line": 0
                        }
                    },
                    "selectionRange": {
                        "end": {
                        "character": 1,
                        "line": 2
                        },
                        "start": {
                        "character": 0,
                        "line": 0
                        }
                    },
                    "tags": [

                    ]
                    }
                ]
            ),
        );
        // // std::thread::sleep(Duration::new(1, 0));
        // log::info!("\n------------------------------\n");
        eprintln!("actual_r = {:?}", actual_r.result);
        eprintln!("\n\n\n");
        // log::trace!("expect_r = {:?}", expect_r);
        log::info!("\n------------------------------\n");
        assert_eq!(actual_r.result, expect_r.result);
    }

    /// Document Symbol 2
    #[test]
    fn test_on_document_symbol_request_002() {
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
            "textDocument": {
                "uri": "file://".to_string() + fpath.to_str().unwrap()
            },
        });
        let request = Request {
            id: "on_document_symbol_request_002".to_string().into(),
            method: String::from("textDocument/documentSymbol"),
            params: params_json,
        };

        let actual_r = on_document_symbol_request(&context, &request);
        let expect_r = Response::new_ok(
            "on_document_symbol_request_002".to_string().into(),
            json!(
                [{
                    "children": [{
                        "deprecated": false,
                        "kind": 12,
                        "name": "initialize_lp_account",
                        "range": {
                            "end": {
                                "character": 5,
                                "line": 28
                            },
                            "start": {
                                "character": 4,
                                "line": 14
                            }
                        },
                        "selectionRange": {
                            "end": {
                                "character": 5,
                                "line": 28
                            },
                            "start": {
                                "character": 4,
                                "line": 14
                            }
                        },
                        "tags": []
                    }, {
                        "deprecated": false,
                        "kind": 12,
                        "name": "retrieve_signer_cap",
                        "range": {
                            "end": {
                                "character": 5,
                                "line": 36
                            },
                            "start": {
                                "character": 4,
                                "line": 32
                            }
                        },
                        "selectionRange": {
                            "end": {
                                "character": 5,
                                "line": 36
                            },
                            "start": {
                                "character": 4,
                                "line": 32
                            }
                        },
                        "tags": []
                    }, {
                        "deprecated": false,
                        "kind": 14,
                        "name": "ERR_FORBIDDEN",
                        "range": {
                            "end": {
                                "character": 35,
                                "line": 7
                            },
                            "start": {
                                "character": 4,
                                "line": 7
                            }
                        },
                        "selectionRange": {
                            "end": {
                                "character": 35,
                                "line": 7
                            },
                            "start": {
                                "character": 4,
                                "line": 7
                            }
                        },
                        "tags": []
                    }, {
                        "children": [{
                            "deprecated": false,
                            "kind": 8,
                            "name": "signer_cap",
                            "range": {
                                "end": {
                                    "character": 49,
                                    "line": 10
                                },
                                "start": {
                                    "character": 39,
                                    "line": 10
                                }
                            },
                            "selectionRange": {
                                "end": {
                                    "character": 49,
                                    "line": 10
                                },
                                "start": {
                                    "character": 39,
                                    "line": 10
                                }
                            },
                            "tags": []
                        }],
                        "deprecated": false,
                        "kind": 14,
                        "name": "CapabilityStorage",
                        "range": {
                            "end": {
                                "character": 69,
                                "line": 10
                            },
                            "start": {
                                "character": 4,
                                "line": 10
                            }
                        },
                        "selectionRange": {
                            "end": {
                                "character": 69,
                                "line": 10
                            },
                            "start": {
                                "character": 4,
                                "line": 10
                            }
                        },
                        "tags": []
                    }],
                    "deprecated": false,
                    "detail": "LPResourceAccount",
                    "kind": 2,
                    "name": "LPResourceAccount",
                    "range": {
                        "end": {
                            "character": 1,
                            "line": 37
                        },
                        "start": {
                            "character": 0,
                            "line": 1
                        }
                    },
                    "selectionRange": {
                        "end": {
                            "character": 1,
                            "line": 37
                        },
                        "start": {
                            "character": 0,
                            "line": 1
                        }
                    },
                    "tags": []
                }]
            ),
        );

        assert_eq!(actual_r.result, expect_r.result);
    }
}
