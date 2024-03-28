// Copyright (c) The BitsLab.Move Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::{
    analyzer_handler::*,
    context::*,
    utils::{path_concat, FileRange},
};
use lsp_server::*;
use lsp_types::*;
use move_model::{
    ast::{ExpData::*, Operation::*, Pattern as MoveModelPattern, Spec, SpecBlockTarget},
    model::{FunId, GlobalEnv, ModuleEnv, ModuleId, NodeId, StructId},
    symbol::Symbol,
};
use std::{
    collections::HashMap,
    ops::Deref,
    path::{Path, PathBuf},
};

/// Handles go-to-def request of the language server.
pub fn on_go_to_def_request(context: &Context, request: &Request) -> lsp_server::Response {
    log::info!("on_go_to_def_request request = {:?}", request);
    let parameters = serde_json::from_value::<GotoDefinitionParams>(request.params.clone())
        .expect("could not deserialize go-to-def request");
    let fpath = parameters
        .text_document_position_params
        .text_document
        .uri
        .to_file_path()
        .unwrap();
    let loc = parameters.text_document_position_params.position;
    let line = loc.line;
    let col = loc.character;
    let fpath = path_concat(std::env::current_dir().unwrap().as_path(), fpath.as_path());

    let project = match context.projects.get_project(&fpath) {
        Some(x) => x,
        None => {
            log::error!("project not found:{:?}", fpath.as_path());
            return Response {
                id: "".to_string().into(),
                result: Some(serde_json::json!({"msg": "No available project"})),
                error: None,
            };
        },
    };
    let mut handler = Handler::new(fpath.clone(), line, col);
    handler.addrname_2_addrnum = project.addrname_2_addrnum.clone();
    project.run_visitor_for_file(&mut handler, &fpath, String::default());

    handler.remove_not_in_loc(&project.global_env);

    let locations = handler.convert_to_locations();

    let r = Response::new_ok(
        request.id.clone(),
        serde_json::to_value(GotoDefinitionResponse::Array(locations)).unwrap(),
    );
    let ret_response = r.clone();
    context
        .connection
        .sender
        .send(Message::Response(r))
        .unwrap();
    log::trace!("goto definition Success");
    ret_response
}

pub(crate) struct Handler {
    /// The file we are looking for.
    pub(crate) filepath: PathBuf,
    pub(crate) line: u32,
    pub(crate) col: u32,
    pub(crate) result: Option<FileRange>,

    pub(crate) mouse_span: codespan::Span,
    pub(crate) capture_items_span: Vec<codespan::Span>,
    pub(crate) result_candidates: Vec<FileRange>,
    pub(crate) target_module_id: ModuleId,
    pub(crate) symbol_2_pattern_id: HashMap<Symbol, NodeId>, // LocalVar => Block::Pattern, only remeber the last pattern
    pub(crate) addrname_2_addrnum: HashMap<String, String>,
}

impl Handler {
    pub(crate) fn new(filepath: impl Into<PathBuf>, line: u32, col: u32) -> Self {
        Self {
            filepath: filepath.into(),
            line,
            col,
            result: None,
            mouse_span: Default::default(),
            capture_items_span: vec![],
            result_candidates: vec![],
            target_module_id: ModuleId::new(0),
            symbol_2_pattern_id: HashMap::new(),
            addrname_2_addrnum: HashMap::new(),
        }
    }

    fn check_move_model_loc_contains_mouse_pos(
        &self,
        env: &GlobalEnv,
        loc: &move_model::model::Loc,
    ) -> bool {
        if let Some(obj_first_col) = env.get_location(&move_model::model::Loc::new(
            loc.file_id(),
            codespan::Span::new(
                loc.span().start(),
                loc.span().start() + codespan::ByteOffset(1),
            ),
        )) {
            if let Some(obj_last_col) = env.get_location(&move_model::model::Loc::new(
                loc.file_id(),
                codespan::Span::new(loc.span().end(), loc.span().end() + codespan::ByteOffset(1)),
            )) {
                if u32::from(obj_first_col.line) == self.line
                    && u32::from(obj_first_col.column) < self.col
                    && self.col < u32::from(obj_last_col.column)
                {
                    return true;
                }
            }
        }
        false
    }

    fn convert_to_locations(&mut self) -> Vec<Location> {
        if let Some(most_clost_item_idx) = find_smallest_length_index(&self.capture_items_span) {
            if most_clost_item_idx < self.result_candidates.len() {
                self.result = Some(self.result_candidates[most_clost_item_idx].clone());
            }
        }

        let mut ret = Vec::with_capacity(2);
        if let Some(x) = self.result.as_ref() {
            ret.push(x.mk_location());
            log::info!(
                "goto definition result path = {}, line = {}",
                x.path.display(),
                x.line_start
            );
        }

        self.capture_items_span.clear();
        self.result_candidates.clear();
        ret
    }

    fn get_mouse_loc(&mut self, env: &GlobalEnv, target_fn_or_struct_loc: &move_model::model::Loc) {
        let mut mouse_line_first_col = move_model::model::Loc::new(
            target_fn_or_struct_loc.file_id(),
            codespan::Span::new(
                target_fn_or_struct_loc.span().start(),
                target_fn_or_struct_loc.span().start() + codespan::ByteOffset(1),
            ),
        );

        let mut mouse_loc = env.get_location(&mouse_line_first_col).unwrap();
        // locate to self.line first column
        while mouse_loc.line.0 < self.line {
            mouse_line_first_col = move_model::model::Loc::new(
                target_fn_or_struct_loc.file_id(),
                codespan::Span::new(
                    mouse_line_first_col.span().end(),
                    mouse_line_first_col.span().end() + codespan::ByteOffset(1),
                ),
            );
            mouse_loc = env.get_location(&mouse_line_first_col).unwrap();
        }

        // locate to self.line last column
        let mut mouse_line_last_col = move_model::model::Loc::new(
            target_fn_or_struct_loc.file_id(),
            codespan::Span::new(
                mouse_line_first_col.span().end(),
                mouse_line_first_col.span().end() + codespan::ByteOffset(1),
            ),
        );

        mouse_loc = env.get_location(&mouse_line_last_col).unwrap();
        // locate to self.line first column
        while mouse_loc.column.0 < self.col && mouse_loc.line.0 == self.line {
            mouse_line_last_col = move_model::model::Loc::new(
                target_fn_or_struct_loc.file_id(),
                codespan::Span::new(
                    mouse_line_last_col.span().end(),
                    mouse_line_last_col.span().end() + codespan::ByteOffset(1),
                ),
            );
            mouse_loc = env.get_location(&mouse_line_last_col).unwrap();
        }

        let mouse_source = env.get_source(&move_model::model::Loc::new(
            target_fn_or_struct_loc.file_id(),
            codespan::Span::new(
                mouse_line_first_col.span().start(),
                mouse_line_last_col.span().start(),
            ),
        ));
        log::info!("get mouse_source = {:?}", mouse_source);
        self.mouse_span = codespan::Span::new(
            mouse_line_first_col.span().start(),
            mouse_line_last_col.span().start(),
        );
    }

    fn remove_not_in_loc(&mut self, env: &GlobalEnv) {
        let mut res_capture_items_span = vec![];
        let mut res_result_candidates = vec![];
        let mut indexes_to_retain = vec![];
        for index in 0..self.capture_items_span.len() {
            if let Some(file_id) =
                crate::utils::get_file_id_by_fpath_in_all_modules(env, &self.filepath)
            {
                let capture_span = self.capture_items_span.get(index).unwrap();
                let span_loc = move_model::model::Loc::new(
                    file_id,
                    codespan::Span::new(capture_span.start(), capture_span.end()),
                );

                if crate::move_generate_spec_sel::ReqParametersPath::is_linecol_in_loc(
                    self.line, self.col, &span_loc, env,
                ) {
                    indexes_to_retain.push(index)
                }
            }
        }

        for (index, span) in self.capture_items_span.iter().enumerate() {
            if indexes_to_retain.contains(&index) {
                res_capture_items_span.push(*span);
            }
        }
        self.capture_items_span = res_capture_items_span;

        for (index, file_range) in self.result_candidates.iter().enumerate() {
            if indexes_to_retain.contains(&index) {
                res_result_candidates.push(file_range.clone());
            }
        }
        self.result_candidates = res_result_candidates;
    }

    fn process_use_decl(&mut self, env: &GlobalEnv) {
        log::trace!("process_use_decl =======================================\n\n");
        let target_module = env.get_module(self.target_module_id);
        let spool = env.symbol_pool();
        let mut target_stct_or_fn = String::default();
        let mut found_target_stct_or_fn = false;
        let mut found_usedecl_same_line = false;
        let mut capture_items_loc = move_model::model::Loc::default();
        let mut addrnum_with_module_name = Default::default();

        for use_decl in target_module.get_use_decls() {
            if !self.check_move_model_loc_contains_mouse_pos(env, &use_decl.loc) {
                continue;
            }
            log::info!(
                "find use decl module, line: {}",
                use_decl.loc.span().start()
            );

            let used_module_name = use_decl.module_name.display_full(env).to_string();
            let before_after = used_module_name.split("::").collect::<Vec<_>>();
            if before_after.len() != 2 {
                log::error!("use decl module name len should be 2");
                continue;
            }

            let addrnum = match self.addrname_2_addrnum.get(&before_after[0].to_string()) {
                Some(x) => x,
                None => {
                    log::error!("could not convert addrname to addrnum, please check you use decl");
                    continue;
                },
            };

            addrnum_with_module_name = addrnum.clone() + "::" + before_after[1];
            found_usedecl_same_line = true;
            capture_items_loc = use_decl.loc.clone();

            if !use_decl.members.is_empty() {
                for (member_loc, name, _alias_name) in use_decl.members.clone().into_iter() {
                    log::trace!("member_loc = {:?} ---", env.get_location(&member_loc));
                    if self.check_move_model_loc_contains_mouse_pos(env, &member_loc) {
                        target_stct_or_fn = name.display(spool).to_string();
                        found_target_stct_or_fn = true;
                        capture_items_loc = member_loc;
                        log::info!("find use decl member {}", target_stct_or_fn);
                        break;
                    }
                }
            }
            if found_target_stct_or_fn {
                break;
            }
        }

        if !found_target_stct_or_fn && !found_usedecl_same_line {
            return;
        }

        let mut option_use_module: Option<ModuleEnv<'_>> = None;
        for mo_env in env.get_modules() {
            let mo_name_str = mo_env.get_name().display_full(env).to_string();
            if addrnum_with_module_name.len() != mo_name_str.len() {
                continue;
            }

            if mo_name_str.to_lowercase() == addrnum_with_module_name.to_lowercase() {
                option_use_module = Some(mo_env);
                break;
            }
        }

        let use_decl_module = match option_use_module {
            Some(x) => x,
            None => return,
        };

        if found_target_stct_or_fn {
            log::info!("finding use decl module member...");
            for stct in use_decl_module.get_structs() {
                log::trace!(
                    "per_struct_name = {:?}, target_struct: {}",
                    stct.get_full_name_str(),
                    target_stct_or_fn
                );
                if stct.get_full_name_str().contains(&target_stct_or_fn) {
                    log::info!("stct.get_full_name_str() = {:?}", stct.get_full_name_str());
                    let (type_struct_file, type_struct_pos) =
                        env.get_file_and_location(&stct.get_loc()).unwrap();
                    let result = FileRange {
                        path: PathBuf::from(type_struct_file),
                        line_start: type_struct_pos.line.0,
                        col_start: type_struct_pos.column.0,
                        line_end: type_struct_pos.line.0,
                        col_end: type_struct_pos.column.0,
                    };
                    self.capture_items_span.push(capture_items_loc.span());
                    self.result_candidates.push(result);
                    return;
                }
            }
            for func in use_decl_module.get_functions() {
                log::trace!(
                    "per_fun_name = {:?}, target_fun: {}",
                    func.get_full_name_str(),
                    target_stct_or_fn
                );
                if func.get_name_str().contains(&target_stct_or_fn) {
                    log::info!("func.get_name_str() = {:?}", func.get_name_str());
                    let (use_fun_file, use_fun_line) =
                        env.get_file_and_location(&func.get_loc()).unwrap();
                    let path_buf = PathBuf::from(use_fun_file);
                    let result = FileRange {
                        path: path_buf,
                        line_start: use_fun_line.line.0,
                        col_start: use_fun_line.column.0,
                        line_end: use_fun_line.line.0,
                        col_end: use_fun_line.column.0,
                    };
                    self.capture_items_span.push(capture_items_loc.span());
                    self.result_candidates.push(result);
                    return;
                }
            }
        }

        if found_usedecl_same_line {
            log::info!("find use decl module...");
            let (module_file, module_pos) = env
                .get_file_and_location(&use_decl_module.get_loc())
                .unwrap();
            let result = FileRange {
                path: PathBuf::from(module_file),
                line_start: module_pos.line.0,
                col_start: module_pos.column.0,
                line_end: module_pos.line.0,
                col_end: module_pos.column.0,
            };
            self.capture_items_span.push(capture_items_loc.span());
            self.result_candidates.push(result);
        }
    }

    fn process_func(&mut self, env: &GlobalEnv) {
        log::trace!("process_func =======================================\n\n");
        let mut found_target_fun = false;
        let mut target_fun_id = FunId::new(env.symbol_pool().make("name"));

        let target_module = env.get_module(self.target_module_id);
        for fun in target_module.get_functions() {
            let this_fun_loc = fun.get_loc();
            let (_, func_start_pos) = env.get_file_and_location(&this_fun_loc).unwrap();
            let (_, func_end_pos) = env
                .get_file_and_location(&move_model::model::Loc::new(
                    this_fun_loc.file_id(),
                    codespan::Span::new(this_fun_loc.span().end(), this_fun_loc.span().end()),
                ))
                .unwrap();

            if func_start_pos.line.0 <= self.line && self.line < func_end_pos.line.0 {
                log::info!(
                    "get target function: func_start_pos = {:?}, func_end_pos = {:?}",
                    func_start_pos,
                    func_end_pos
                );
                target_fun_id = fun.get_id();
                found_target_fun = true;
                break;
            }
        }

        if !found_target_fun {
            return;
        }

        let target_module = env.get_module(self.target_module_id);
        let target_fun = target_module.get_function(target_fun_id);
        let target_fun_loc: move_model::model::Loc = target_fun.get_loc();
        self.get_mouse_loc(env, &target_fun_loc);
        self.process_parameter();
        self.process_return_type();

        if let Some(exp) = target_fun.get_def().deref() {
            self.process_expr(env, exp);
        };
    }

    /// MoveModel currently lacks support for obtaining the Span and Loc of function parameters and return type.
    /// As a result, it is unable to compare positions with user-clicked addresses.
    /// Parameters and return types in function signatures are currently not supported for navigation.
    fn process_parameter(&mut self) {}

    fn process_return_type(&mut self) {}

    fn process_spec_func(&mut self, env: &GlobalEnv) {
        log::trace!("process_spec_func =======================================");
        let mut found_target_fun = false;
        let mut target_fun_id = FunId::new(env.symbol_pool().make("name"));
        let target_module = env.get_module(self.target_module_id);
        let mut spec_fn_span_loc = target_module.get_loc();

        for spec_block_info in target_module.get_spec_block_infos() {
            if let SpecBlockTarget::Function(_, fun_id) = spec_block_info.target {
                let span_first_col = move_model::model::Loc::new(
                    spec_block_info.loc.file_id(),
                    codespan::Span::new(
                        spec_block_info.loc.span().start(),
                        spec_block_info.loc.span().start() + codespan::ByteOffset(1),
                    ),
                );
                let span_last_col = move_model::model::Loc::new(
                    spec_block_info.loc.file_id(),
                    codespan::Span::new(
                        spec_block_info.loc.span().end(),
                        spec_block_info.loc.span().end() + codespan::ByteOffset(1),
                    ),
                );

                if let Some(s_loc) = env.get_location(&span_first_col) {
                    if let Some(e_loc) = env.get_location(&span_last_col) {
                        if u32::from(s_loc.line) <= self.line && self.line <= u32::from(e_loc.line)
                        {
                            target_fun_id = fun_id;
                            found_target_fun = true;
                            spec_fn_span_loc = spec_block_info.loc.clone();
                            break;
                        }
                    }
                }
            }
        }

        if !found_target_fun {
            log::trace!("<< not found_target_spec_fun");
            return;
        }

        let target_fn = target_module.get_function(target_fun_id);
        let target_fn_spec = target_fn.get_spec();
        log::info!("target_fun's spec = {}", env.display(&*target_fn_spec));
        self.get_mouse_loc(env, &spec_fn_span_loc);
        for cond in target_fn_spec.conditions.clone() {
            for exp in cond.all_exps() {
                self.process_expr(env, exp);
            }
        }
    }

    fn process_struct(&mut self, env: &GlobalEnv) {
        log::trace!("process_struct =======================================\n\n");
        let mut found_target_struct = false;
        let mut target_struct_id = StructId::new(env.symbol_pool().make("name"));
        let target_module = env.get_module(self.target_module_id);
        for struct_env in target_module.get_structs() {
            let struct_loc = struct_env.get_loc();
            let (_, struct_start_pos) = env.get_file_and_location(&struct_loc).unwrap();
            let (_, struct_end_pos) = env
                .get_file_and_location(&move_model::model::Loc::new(
                    struct_loc.file_id(),
                    codespan::Span::new(struct_loc.span().end(), struct_loc.span().end()),
                ))
                .unwrap();
            if struct_start_pos.line.0 < self.line && self.line < struct_end_pos.line.0 {
                target_struct_id = struct_env.get_id();
                found_target_struct = true;
                break;
            }
        }
        if !found_target_struct {
            return;
        }

        let target_module = env.get_module(self.target_module_id);
        let target_struct = target_module.get_struct(target_struct_id);
        let target_struct_loc = target_struct.get_loc();
        self.get_mouse_loc(env, &target_struct_loc);

        for field_env in target_struct.get_fields() {
            let field_name = field_env.get_name();
            let field_name_str = field_name.display(env.symbol_pool());
            log::trace!("field_name = {}", field_name_str);
            let struct_source = env.get_source(&target_struct_loc);
            if let Ok(struct_str) = struct_source {
                if let Some(index) = struct_str.find(field_name_str.to_string().as_str()) {
                    let field_len = field_name_str.to_string().len();
                    let field_start = target_struct_loc.span().start()
                        + codespan::ByteOffset((index + field_len).try_into().unwrap());
                    // Assuming a relatively large distance
                    let field_end = field_start + codespan::ByteOffset((128).try_into().unwrap());
                    let field_loc = move_model::model::Loc::new(
                        target_struct_loc.file_id(),
                        codespan::Span::new(field_start, field_end),
                    );
                    let field_source = env.get_source(&field_loc);
                    if let Ok(atomic_field_str) = field_source {
                        if let Some(index) = atomic_field_str.find("\n".to_string().as_str()) {
                            let atomic_field_end =
                                field_start + codespan::ByteOffset(index.try_into().unwrap());
                            let atomic_field_loc = move_model::model::Loc::new(
                                target_struct_loc.file_id(),
                                codespan::Span::new(field_start, atomic_field_end),
                            );
                            let atomic_field_source = env.get_source(&atomic_field_loc);
                            if atomic_field_loc.span().end() < self.mouse_span.end()
                                || atomic_field_loc.span().start() > self.mouse_span.end()
                            {
                                continue;
                            }
                            log::info!("atomic_field_source = {:?}", atomic_field_source);
                            let field_type = field_env.get_type();
                            self.process_type(env, &atomic_field_loc, &field_type);
                        }
                    }
                }
            }
        }
    }

    fn process_spec_struct(&mut self, env: &GlobalEnv) {
        log::trace!("process_spec_struct =======================================\n\n");
        let mut found_target_spec_stct = false;
        let mut target_stct_id = StructId::new(env.symbol_pool().make("name"));
        let target_module = env.get_module(self.target_module_id);
        let mut spec_stct_span_loc = target_module.get_loc();

        for spec_block_info in target_module.get_spec_block_infos() {
            if let SpecBlockTarget::Struct(_, stct_id) = spec_block_info.target {
                let span_first_col = move_model::model::Loc::new(
                    spec_block_info.loc.file_id(),
                    codespan::Span::new(
                        spec_block_info.loc.span().start(),
                        spec_block_info.loc.span().start() + codespan::ByteOffset(1),
                    ),
                );
                let span_last_col = move_model::model::Loc::new(
                    spec_block_info.loc.file_id(),
                    codespan::Span::new(
                        spec_block_info.loc.span().end(),
                        spec_block_info.loc.span().end() + codespan::ByteOffset(1),
                    ),
                );

                if let Some(s_loc) = env.get_location(&span_first_col) {
                    if let Some(e_loc) = env.get_location(&span_last_col) {
                        if u32::from(s_loc.line) <= self.line && self.line <= u32::from(e_loc.line)
                        {
                            target_stct_id = stct_id;
                            found_target_spec_stct = true;
                            spec_stct_span_loc = spec_block_info.loc.clone();
                            break;
                        }
                    }
                }
            }
        }

        if !found_target_spec_stct {
            log::trace!("<< not found_target_spec_stct");
            return;
        }

        let target_stct = target_module.get_struct(target_stct_id);
        let target_stct_spec = target_stct.get_spec();
        log::info!("target_stct's spec = {}", env.display(target_stct_spec));
        self.get_mouse_loc(env, &spec_stct_span_loc);
        for cond in target_stct_spec.conditions.clone() {
            for exp in cond.all_exps() {
                self.process_expr(env, exp);
            }
        }
    }

    fn process_expr(&mut self, env: &GlobalEnv, exp: &move_model::ast::Exp) {
        log::trace!("process_expr -------------------------\n");

        exp.visit_post_order(&mut |e| {
            match e {
                Value(node_id, _) => {
                    // Const variable
                    let value_loc = env.get_node_loc(*node_id);
                    if self.check_move_model_loc_contains_mouse_pos(env, &value_loc) {
                        let mut value_str = String::default();
                        if let Ok(capture_value_str) = env.get_source(&value_loc) {
                            value_str = capture_value_str.to_string();
                        }
                        for named_const in
                            env.get_module(self.target_module_id).get_named_constants()
                        {
                            let spool = env.symbol_pool();
                            log::trace!(
                                "named_const.get_name() = {}",
                                named_const.get_name().display(spool)
                            );
                            let named_const_str = named_const.get_name().display(spool).to_string();
                            if value_str.contains(&named_const_str) {
                                let (def_file, def_line) =
                                    env.get_file_and_location(&named_const.get_loc()).unwrap();
                                let result = FileRange {
                                    path: PathBuf::from(def_file),
                                    line_start: def_line.line.0,
                                    col_start: def_line.column.0,
                                    line_end: def_line.line.0,
                                    col_end: def_line.column.0,
                                };
                                self.capture_items_span.push(value_loc.span());
                                self.result_candidates.push(result);
                            }
                        }
                    }
                    true
                },
                LocalVar(node_id, localvar_symbol) => {
                    let localvar_loc = env.get_node_loc(*node_id);

                    if localvar_loc.span().start() > self.mouse_span.end()
                        || localvar_loc.span().end() < self.mouse_span.end()
                    {
                        return true;
                    }
                    log::trace!(
                        "target: exp.visit localvar_symbol = {}",
                        localvar_symbol.display(env.symbol_pool())
                    );
                    if let Some(pattern_id) = self.symbol_2_pattern_id.get(localvar_symbol) {
                        let pattern_loc = env.get_node_loc(*pattern_id);
                        if let Some((def_file, def_location)) =
                            env.get_file_and_location(&pattern_loc)
                        {
                            let result = FileRange {
                                path: PathBuf::from(def_file),
                                line_start: def_location.line.0,
                                col_start: def_location.column.0,
                                line_end: def_location.line.0,
                                col_end: def_location.column.0,
                            };
                            self.capture_items_span.push(localvar_loc.span());
                            self.result_candidates.push(result);
                        }
                    }
                    true
                },
                Temporary(node_id, _) => {
                    let tmpvar_loc = env.get_node_loc(*node_id);
                    if let Ok(x) = env.get_source(&tmpvar_loc) {
                        log::trace!("temp_source = {}", x)
                    }

                    if tmpvar_loc.span().start() > self.mouse_span.end() {
                        log::trace!("tmpvar_loc.span().start() > self.mouse_span.end()");
                        return true;
                    }

                    if tmpvar_loc.span().end() < self.mouse_span.end() {
                        log::trace!("tmpvar_loc.span().end() < self.mouse_span.end()");
                        return true;
                    }
                    log::trace!("Temporary is available");
                    if let Some(node_type) = env.get_node_type_opt(*node_id) {
                        self.process_type(env, &tmpvar_loc, &node_type);
                    }
                    true
                },
                Call(..) => {
                    log::trace!("lll >> exp.visit Call",);
                    self.process_call_spec_func(env, e);
                    self.process_call(env, e);
                    true
                },
                Return(_, ret_exp) => {
                    self.process_expr(env, ret_exp);
                    true
                },
                Sequence(_, exp_vec) => {
                    for seq_exp in exp_vec {
                        self.process_expr(env, seq_exp);
                    }
                    true
                },
                SpecBlock(node_id, spec) => {
                    self.process_spec_block(env, &env.get_node_loc(*node_id), spec);
                    true
                },
                Block(_, pat, left_exp, right_exp) => {
                    self.collect_local_var_in_pattern(pat);
                    self.process_pattern(env, pat);

                    if let Some(l_exp) = left_exp {
                        self.process_expr(env, l_exp);
                    }
                    self.process_expr(env, right_exp);
                    true
                },
                _ => {
                    log::trace!("________________");
                    true
                },
            }
        });
        log::trace!("\nlll << process_expr ^^^^^^^^^^^^^^^^^^^^^^^^^\n");
    }

    fn process_call_spec_func(&mut self, env: &GlobalEnv, expdata: &move_model::ast::ExpData) {
        if let Call(node_id, SpecFunction(mid, fid, _), _) = expdata {
            let this_call_loc = env.get_node_loc(*node_id);
            log::trace!(
                "lll >> exp.visit this_call_loc = {:?}",
                env.get_file_and_location(&this_call_loc)
            );
            if this_call_loc.span().start() < self.mouse_span.end()
                && self.mouse_span.end() < this_call_loc.span().end()
            {
                let called_module = env.get_module(*mid);
                let spec_fun = called_module.get_spec_fun(*fid);
                log::trace!(
                    "lll >> get_spec_functions = {}",
                    spec_fun.name.display(env.symbol_pool())
                );
                let spec_fun_loc = spec_fun.loc.clone();
                let (spec_fun_file, spec_fun_line) =
                    env.get_file_and_location(&spec_fun_loc).unwrap();
                let path_buf = PathBuf::from(spec_fun_file);
                let result = FileRange {
                    path: path_buf,
                    line_start: spec_fun_line.line.0,
                    col_start: spec_fun_line.column.0,
                    line_end: spec_fun_line.line.0,
                    col_end: spec_fun_line.column.0,
                };
                self.result_candidates.push(result);
                self.capture_items_span.push(this_call_loc.span());

                let inst_vec = &env.get_node_instantiation(*node_id);
                for inst in inst_vec {
                    let mut generic_ty_loc = this_call_loc.clone();
                    let capture_call_source = env.get_source(&this_call_loc);
                    if let Ok(capture_call_source_str) = capture_call_source {
                        if let Some(index) = capture_call_source_str.find("<".to_string().as_str())
                        {
                            generic_ty_loc = move_model::model::Loc::new(
                                this_call_loc.file_id(),
                                codespan::Span::new(
                                    this_call_loc.span().start()
                                        + codespan::ByteOffset(index.try_into().unwrap()),
                                    this_call_loc.span().end(),
                                ),
                            );
                        }
                    }
                    self.process_type(env, &generic_ty_loc, inst);
                }
            }
        }
        if let Call(node_id, Exists(..), exp_vec) = expdata {
            let this_call_loc = env.get_node_loc(*node_id);
            log::trace!(
                "lll >> exp.visit Exists this_call_loc = {:?}",
                env.get_file_and_location(&this_call_loc)
            );
            if this_call_loc.span().start() < self.mouse_span.end()
                && self.mouse_span.end() < this_call_loc.span().end()
            {
                log::trace!("lll >> exp.visit Exists exist_exp = {:?}", exp_vec);
                for exist_exp in exp_vec {
                    self.process_expr(env, exist_exp);
                }
            }
        }
    }

    fn process_call(&mut self, env: &GlobalEnv, expdata: &move_model::ast::ExpData) {
        log::trace!("lll >> process_call");
        if let Call(node_id, MoveFunction(mid, fid), _) = expdata {
            let this_call_loc = env.get_node_loc(*node_id);
            log::trace!(
                "lll >> exp.visit this_call_loc = {:?}",
                env.get_location(&this_call_loc)
            );
            if this_call_loc.span().start() < self.mouse_span.end()
                && self.mouse_span.end() < this_call_loc.span().end()
            {
                let called_module = env.get_module(*mid);
                let called_fun = called_module.get_function(*fid);
                log::trace!(
                    " get_called_functions = {:?}",
                    called_fun.get_full_name_str()
                );
                let called_fun_loc = called_fun.get_loc();
                let (called_fun_file, called_fun_line) =
                    env.get_file_and_location(&called_fun_loc).unwrap();
                let path_buf = PathBuf::from(called_fun_file);
                let result = FileRange {
                    path: path_buf,
                    line_start: called_fun_line.line.0,
                    col_start: called_fun_line.column.0,
                    line_end: called_fun_line.line.0,
                    col_end: called_fun_line.column.0 + called_fun.get_full_name_str().len() as u32,
                };
                self.result_candidates.push(result);
                self.capture_items_span.push(this_call_loc.span());

                let inst_vec = &env.get_node_instantiation(*node_id);
                for inst in inst_vec {
                    let mut generic_ty_loc = this_call_loc.clone();
                    let capture_call_source = env.get_source(&this_call_loc);
                    if let Ok(capture_call_source_str) = capture_call_source {
                        if let Some(index) = capture_call_source_str.find("<".to_string().as_str())
                        {
                            generic_ty_loc = move_model::model::Loc::new(
                                this_call_loc.file_id(),
                                codespan::Span::new(
                                    this_call_loc.span().start()
                                        + codespan::ByteOffset(index.try_into().unwrap()),
                                    this_call_loc.span().end(),
                                ),
                            );
                        }
                    }
                    self.process_type(env, &generic_ty_loc, inst);
                }
            }
        }

        if let Call(node_id, BorrowGlobal(..), _) = expdata {
            let this_call_loc = env.get_node_loc(*node_id);
            log::trace!(
                "lll >> exp.visit this_call_loc = {:?}",
                env.get_location(&this_call_loc)
            );
            if this_call_loc.span().start() < self.mouse_span.end()
                && self.mouse_span.end() < this_call_loc.span().end()
            {
                let inst_vec = &env.get_node_instantiation(*node_id);
                for inst in inst_vec {
                    log::info!("lll >> inst = {:?}", inst);
                    self.process_type(env, &this_call_loc, inst);
                }
            }
        }

        if let Call(node_id, Select(mid, sid, fid), _) = expdata {
            let this_call_loc = env.get_node_loc(*node_id);
            log::trace!(
                "lll >> exp.visit this_call_loc = {:?}",
                env.get_location(&this_call_loc)
            );
            if this_call_loc.span().start() > self.mouse_span.end()
                || self.mouse_span.end() > this_call_loc.span().end()
            {
                return;
            }
            let called_module = env.get_module(*mid);
            let called_struct = called_module.get_struct(*sid);
            log::trace!(
                "lll >> called_struct = {:?}",
                called_struct.get_full_name_str()
            );
            let called_field = called_struct.get_field(*fid);
            let field_name = called_field.get_name();
            let field_name_str = field_name.display(env.symbol_pool());

            let called_struct_loc = called_struct.get_loc();
            let call_struct_source = env.get_source(&called_struct_loc);
            if let Ok(call_struct_str) = call_struct_source {
                if let Some(index) = call_struct_str.find(field_name_str.to_string().as_str()) {
                    let field_start = called_struct_loc.span().start()
                        + codespan::ByteOffset(index.try_into().unwrap());
                    let field_len = field_name_str.to_string().len();
                    let field_end =
                        field_start + codespan::ByteOffset(field_len.try_into().unwrap());
                    let field_loc = move_model::model::Loc::new(
                        called_struct_loc.file_id(),
                        codespan::Span::new(field_start, field_end),
                    );
                    let call_field_source = env.get_source(&field_loc);
                    log::trace!("lll >> call_field_source = {:?}", call_field_source);
                    let (called_field_file, called_field_line) =
                        env.get_file_and_location(&field_loc).unwrap();
                    let path_buf = PathBuf::from(called_field_file);
                    let result = FileRange {
                        path: path_buf,
                        line_start: called_field_line.line.0,
                        col_start: called_field_line.column.0,
                        line_end: called_field_line.line.0,
                        col_end: called_field_line.column.0,
                    };
                    self.result_candidates.push(result);
                    self.capture_items_span.push(this_call_loc.span());
                }
            }
        }

        if let Call(node_id, Pack(mid, sid), _) = expdata {
            let this_call_loc = env.get_node_loc(*node_id);
            log::trace!(
                "lll >> exp.visit this_call_loc = {:?}",
                env.get_location(&this_call_loc)
            );
            if this_call_loc.span().start() > self.mouse_span.end()
                || self.mouse_span.end() > this_call_loc.span().end()
            {
                return;
            }
            let pack_module = env.get_module(*mid);
            let pack_struct = pack_module.get_struct(*sid);
            log::trace!("lll >> pack_struct = {:?}", pack_struct.get_full_name_str());

            let pack_struct_loc = pack_struct.get_loc();

            let (pack_struct_file, pack_struct_location) =
                env.get_file_and_location(&pack_struct_loc).unwrap();

            let path_buf = PathBuf::from(pack_struct_file);
            let result = FileRange {
                path: path_buf,
                line_start: pack_struct_location.line.0,
                col_start: pack_struct_location.column.0,
                line_end: pack_struct_location.line.0,
                col_end: pack_struct_location.column.0
                    + pack_struct.get_full_name_str().len() as u32,
            };
            self.result_candidates.push(result);
            self.capture_items_span.push(this_call_loc.span());
        }
        log::trace!("lll << process_call");
    }

    fn collect_local_var_in_pattern(&mut self, pattern: &MoveModelPattern) {
        for (node_id, sym) in pattern.vars().iter() {
            self.symbol_2_pattern_id.insert(*sym, *node_id);
        }
    }

    fn process_pattern(&mut self, env: &GlobalEnv, pattern: &MoveModelPattern) {
        if let MoveModelPattern::Struct(node_id, q_id, _) = pattern {
            let this_call_loc = env.get_node_loc(*node_id);
            log::trace!(
                "lll >> exp.visit this_call_loc = {:?}",
                env.get_location(&this_call_loc)
            );
            if this_call_loc.span().start() > self.mouse_span.end()
                || self.mouse_span.end() > this_call_loc.span().end()
            {
                return;
            }
            let pattern_module = env.get_module(q_id.module_id);
            let pattern_struct = pattern_module.get_struct(q_id.id);
            log::info!("pattern_struct = {:?}", pattern_struct.get_full_name_str());

            let pattern_struct_loc = pattern_struct.get_loc();

            let (pattern_struct_file, pattern_struct_location) =
                env.get_file_and_location(&pattern_struct_loc).unwrap();

            let path_buf = PathBuf::from(pattern_struct_file);
            let result = FileRange {
                path: path_buf,
                line_start: pattern_struct_location.line.0,
                col_start: pattern_struct_location.column.0,
                line_end: pattern_struct_location.line.0,
                col_end: pattern_struct_location.column.0
                    + pattern_struct.get_full_name_str().len() as u32,
            };
            self.result_candidates.push(result);
            self.capture_items_span.push(this_call_loc.span());
        }
    }

    fn process_spec_block(
        &mut self,
        env: &GlobalEnv,
        capture_items_loc: &move_model::model::Loc,
        _speck_block: &Spec,
    ) {
        log::trace!("\n\n");

        log::trace!(
            "lll >> process_spec_block loc = {:?}",
            env.get_file_and_location(capture_items_loc)
        );
        log::trace!("lll << process_spec_block loc");
    }

    fn process_type(
        &mut self,
        env: &GlobalEnv,
        capture_items_loc: &move_model::model::Loc,
        ty: &move_model::ty::Type,
    ) {
        log::trace!("process_type ============= ");
        use move_model::ty::Type::*;
        match ty {
            Tuple(..) => {
                log::trace!("lll >> type_var is Tuple");
            },
            Vector(type_ptr) => {
                log::trace!("lll >> type_var is Vector");
                self.process_type(env, capture_items_loc, type_ptr);
            },
            Struct(mid, stid, ty_vec) => {
                let struct_from_module = env.get_module(*mid);
                let type_struct = struct_from_module.get_struct(*stid);
                log::trace!("lll >> type_struct = {:?}", type_struct.get_full_name_str());
                let type_struct_loc = type_struct.get_loc();
                log::trace!(
                    "lll >> type_struct_loc = {:?}",
                    env.get_file_and_location(&type_struct_loc)
                );
                let (type_struct_file, type_struct_pos) =
                    env.get_file_and_location(&type_struct_loc).unwrap();
                let result = FileRange {
                    path: PathBuf::from(type_struct_file),
                    line_start: type_struct_pos.line.0,
                    col_start: type_struct_pos.column.0,
                    line_end: type_struct_pos.line.0,
                    col_end: type_struct_pos.column.0
                        + type_struct.get_full_name_str().len() as u32,
                };
                self.capture_items_span.push((*capture_items_loc).span());
                self.result_candidates.push(result);

                for ty in ty_vec {
                    if let Some(generic_struct_ty) = ty.get_struct(env) {
                        let generic_struct_ty_symbol = generic_struct_ty.0.get_name();
                        let capture_generic_ty_source = env.get_source(capture_items_loc);
                        if let Ok(capture_generic_ty_str) = capture_generic_ty_source {
                            let generic_struct_ty_symbol_display =
                                generic_struct_ty_symbol.display(env.symbol_pool());
                            if let Some(index) = capture_generic_ty_str
                                .find(generic_struct_ty_symbol_display.to_string().as_str())
                            {
                                let capture_generic_ty_str_len =
                                    generic_struct_ty_symbol_display.to_string().len();
                                let capture_generic_ty_start = (*capture_items_loc).span().start()
                                    + codespan::ByteOffset(index.try_into().unwrap());
                                let capture_generic_ty_end = capture_generic_ty_start
                                    + codespan::ByteOffset(
                                        capture_generic_ty_str_len.try_into().unwrap(),
                                    );
                                let capture_generic_ty_loc = move_model::model::Loc::new(
                                    (*capture_items_loc).file_id(),
                                    codespan::Span::new(
                                        capture_generic_ty_start,
                                        capture_generic_ty_end,
                                    ),
                                );
                                log::trace!(
                                    "capture_generic_ty_str = {:?}",
                                    env.get_source(&capture_generic_ty_loc)
                                );
                                let capture_line =
                                    env.get_location(&capture_generic_ty_loc).unwrap();
                                let (generic_struct_belong_file, generic_struct_belong_pos) = env
                                    .get_file_and_location(&generic_struct_ty.0.get_loc())
                                    .unwrap();
                                if self.line.eq(&capture_line.line.0)
                                    && capture_generic_ty_loc.span().start()
                                        <= self.mouse_span.end()
                                    && self.mouse_span.end() <= capture_generic_ty_loc.span().end()
                                {
                                    let result = FileRange {
                                        path: PathBuf::from(generic_struct_belong_file).clone(),
                                        line_start: generic_struct_belong_pos.line.0,
                                        col_start: generic_struct_belong_pos.column.0,
                                        line_end: generic_struct_belong_pos.line.0,
                                        col_end: generic_struct_belong_pos.column.0
                                            + capture_generic_ty_str_len as u32,
                                    };
                                    log::info!("capture_generic_ty result = {:?}", result.clone());
                                    self.capture_items_span.push(capture_generic_ty_loc.span());
                                    self.result_candidates.push(result);
                                }
                            }
                        }
                    }
                }
            },
            TypeParameter(..) => {
                log::trace!("lll >> type_var is TypeParameter");
            },
            Reference(kind, type_ptr) => {
                log::trace!("lll >> type_var is Reference {:?}-{:?}", kind, type_ptr);
                // local_var is Reference Mutable-Struct(ModuleId(37), StructId(Symbol(1531)), [TypeParameter(0)])
                self.process_type(env, capture_items_loc, type_ptr);
            },
            Fun(..) => {
                log::trace!("lll >> type_var is Fun");
            },
            TypeDomain(..) => {
                log::trace!("lll >> type_var is TypeDomain");
            },
            ResourceDomain(..) => {
                log::trace!("lll >> type_var is ResourceDomain");
            },
            Var(..) => {
                log::trace!("lll >> type_var is Var");
            },
            _ => {
                log::trace!("lll >> type_var is default");
            },
        }
    }

    fn run_move_model_visitor_internal(&mut self, env: &GlobalEnv, move_file_path: &Path) {
        let candidate_modules =
            crate::utils::get_modules_by_fpath_in_all_modules(env, &PathBuf::from(move_file_path));
        if candidate_modules.is_empty() {
            log::error!("<goto def>cannot get target module\n");
            return;
        }
        for module_env in candidate_modules.iter() {
            self.target_module_id = module_env.get_id();
            if let Some(s) = move_file_path.to_str() {
                if s.contains(".spec") {
                    self.process_spec_func(env);
                    self.process_spec_struct(env);
                } else {
                    self.process_use_decl(env);
                    self.process_func(env);
                    self.process_struct(env);
                }
            }
        }
    }

    #[allow(unused)]
    fn print_capture_loc_and_source(&mut self, env: &GlobalEnv, span_loc: &move_model::model::Loc) {
        let capture_loc = env.get_location(span_loc).unwrap();
        let a = env.get_source(span_loc).unwrap();
        eprintln!("capture str: {}", a);
        eprintln!(
            "capture loc: {}, {}",
            capture_loc.line.0, capture_loc.column.0
        );
        eprintln!("moutse loc: {}, {}", self.line, self.col);
    }
}

impl ItemOrAccessHandler for Handler {
    fn visit_fun_or_spec_body(&self) -> bool {
        true
    }

    fn finished(&self) -> bool {
        false
    }

    fn handle_project_env(
        &mut self,
        _services: &dyn HandleItemService,
        env: &GlobalEnv,
        move_file_path: &Path,
        _: String,
    ) {
        self.run_move_model_visitor_internal(env, move_file_path);
    }
}

impl std::fmt::Display for Handler {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "goto_definition,file:{:?} line:{} col:{}",
            self.filepath, self.line, self.col
        )
    }
}

pub fn find_smallest_length_index(spans: &[codespan::Span]) -> Option<usize> {
    let mut smallest_length = i64::MAX;
    let mut smallest_index = None;

    for (index, span) in spans.iter().enumerate() {
        let length = span.end() - span.start();
        if length.0 < smallest_length {
            smallest_length = length.0;
            smallest_index = Some(index);
        }
    }

    smallest_index
}
