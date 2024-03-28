// Copyright (c) The BitsLab.MoveBit Contributors
// SPDX-License-Identifier: Apache-2.0

use super::move_generate_spec::*;
use crate::{
    context::Context,
    utils::{collect_use_decl, get_modules_by_fpath_in_target_modules},
};
use lsp_server::*;
use move_model::model::{FunctionEnv, StructEnv};
use serde::Deserialize;
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    str::FromStr,
};

pub fn on_generate_spec_file<'a>(
    context: &Context,
    request: &Request,
    is_generate: bool,
) -> Response
where
    'a: 'static,
{
    log::info!("on_generate_spec_file request = {:?}", request);
    let parameters = serde_json::from_value::<ReqParameters>(request.params.clone())
        .expect("could not deserialize on_generate_spec_file request");
    let fpath = PathBuf::from_str(parameters.fpath.as_str()).unwrap();
    let result_file_path = Resp::mk_result_filepath(&fpath);
    let send_err = |context: &Context, msg: String| {
        let r = Response::new_err(request.id.clone(), ErrorCode::UnknownErrorCode as i32, msg);
        context
            .connection
            .sender
            .send(Message::Response(r))
            .unwrap();
    };
    if result_file_path.exists() {
        send_err(context, "file already exists.".to_string());
        return lsp_server::Response {
            id: "".to_string().into(),
            result: Some(serde_json::json!({"msg": "file already exists."})),
            error: None,
        };
    }
    let project = match context.projects.get_project(&fpath) {
        Some(x) => x,
        None => {
            log::error!("project not found:{:?}", parameters.fpath.as_str());
            return lsp_server::Response {
                id: "".to_string().into(),
                result: Some(serde_json::json!({"msg": "project not found."})),
                error: None,
            };
        },
    };

    let addrname_2_addrnum = &project.addrname_2_addrnum;
    let mut result = ModuleSpecBuilder::new();
    for module_env in get_modules_by_fpath_in_target_modules(&project.global_env, &fpath) {
        let using_module_map = collect_use_decl(
            &project.addrname_2_addrnum,
            &module_env,
            &project.global_env,
        );

        log::info!("generate spec module: {}", module_env.get_full_name_str());
        // find module_env's namespace
        let module_env_full_name = module_env.get_full_name_str();
        let addr_end = module_env_full_name.find("::").unwrap_or_default();
        let addr = module_env_full_name[0..addr_end].to_string();
        let addr_name_default = String::from("0x0");
        let addr_name = addrname_2_addrnum.get(&addr).unwrap_or(&addr_name_default);
        let module_name = module_env_full_name[addr_end + 2..].to_string();

        // find all available StructEnv and FunctionEnv
        let mut env_item_list: Vec<EnvItem> = module_env
            .get_functions()
            .filter(|fun_env| !fun_env.is_test_only())
            .map(|fun_env| EnvItem {
                struct_env: None,
                function_env: Some(fun_env.clone()),
                line: fun_env.get_loc().span().start().0,
            })
            .chain(
                module_env
                    .get_structs()
                    .filter(|struct_env| !struct_env.is_test_only())
                    .map(|struct_env| EnvItem {
                        struct_env: Some(struct_env.clone()),
                        function_env: None,
                        line: struct_env.get_loc().span().start().0,
                    }),
            )
            .collect();

        env_item_list.sort_by(|a, b| a.line.cmp(&b.line));

        for item in env_item_list {
            let spec = match item {
                EnvItem {
                    struct_env: Some(struct_env),
                    function_env: None,
                    ..
                } => genrate_struct_spec(&struct_env),
                EnvItem {
                    struct_env: None,
                    function_env: Some(f_env),
                    ..
                } => generate_fun_spec_zx(
                    &project.global_env,
                    &module_env,
                    &f_env,
                    &using_module_map,
                ),
                _ => continue,
            };

            result.insert(
                AddrAndModuleName::new(addr_name.clone(), module_name.clone()),
                spec,
            );
        }
    } // for module_env

    let file_content = result.get_result_string();
    if is_generate {
        match std::fs::write(result_file_path.clone(), file_content.clone()) {
            Ok(_) => {},
            Err(err) => {
                send_err(context, format!("write to file failed,err:{:?}", err));
                return lsp_server::Response {
                    id: "".to_string().into(),
                    result: Some(serde_json::json!({"msg": "write to file failed"})),
                    error: None,
                };
            },
        };
    }

    let r = Response::new_ok(
        request.id.clone(),
        serde_json::to_value(Resp {
            fpath: result_file_path.to_str().unwrap().to_string(),
        })
        .unwrap(),
    );
    context
        .connection
        .sender
        .send(Message::Response(r))
        .unwrap();

    Response::new_ok(request.id.clone(), serde_json::json!(file_content))
}

#[derive(Debug, Clone)]
struct EnvItem<'a> {
    struct_env: Option<StructEnv<'a>>,
    function_env: Option<FunctionEnv<'a>>,
    line: u32,
}

impl<'a> PartialEq for EnvItem<'a> {
    fn eq(&self, other: &Self) -> bool {
        self.line == other.line
    }
}

impl<'a> Eq for EnvItem<'a> {}

impl<'a> Ord for EnvItem<'a> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.line.cmp(&other.line)
    }
}

impl<'a> PartialOrd for EnvItem<'a> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.line.cmp(&other.line))
    }
}

#[derive(Default)]
struct ModuleSpecBuilder {
    results: HashMap<AddrAndModuleName, Vec<String>>,
}

impl ModuleSpecBuilder {
    fn new() -> Self {
        Self::default()
    }

    fn insert(&mut self, k: AddrAndModuleName, v: String) {
        if let Some(x) = self.results.get_mut(&k) {
            x.push(v);
        } else {
            self.results.insert(k, vec![v]);
        }
    }

    fn get_result_string(self) -> String {
        let mut ret = String::new();
        for (k, vv) in self.results.into_iter() {
            let mut x = String::default();
            x.push_str(
                format!(
                    "spec {}::{} {{\n\n",
                    &k.addr_name.as_str(),
                    &k.module_name.as_str()
                )
                .as_str(),
            );
            x.push_str(format!("{}spec module {{\n", indent(1)).as_str());
            x.push_str(format!("{}pragma verify = true;\n", indent(2)).as_str());
            x.push_str(format!("{}pragma aborts_if_is_strict;\n", indent(2)).as_str());
            x.push_str(format!("{}}}\n", indent(1)).as_str());
            for v in vv.into_iter() {
                x.push_str(v.as_str());
            }
            x.push_str("}\n\n");
            ret.push_str(x.as_str());
        }
        ret
    }
}

#[derive(Clone, Hash, PartialEq, Eq)]
pub struct AddrAndModuleName {
    addr_name: String,
    module_name: String,
}

impl AddrAndModuleName {
    fn new(addr_name: String, module_name: String) -> Self {
        Self {
            addr_name,
            module_name,
        }
    }
}
#[derive(Clone, Deserialize)]
pub struct ReqParameters {
    fpath: String,
}

#[derive(Clone, serde::Serialize)]
pub struct Resp {
    fpath: String,
}

impl Resp {
    fn mk_result_filepath(x: &Path) -> PathBuf {
        let mut x = x.to_path_buf();
        let b = x
            .components()
            .last()
            .and_then(|x| x.as_os_str().to_str())
            .unwrap()
            .to_string();
        let index = b.as_str().rfind('.').unwrap();
        x.pop();
        let mut ret = x.clone();
        ret.push(format!("{}{}", &b.as_str()[0..index], ".spec.move"));
        ret
    }
}
