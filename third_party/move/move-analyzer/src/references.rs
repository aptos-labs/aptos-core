// Copyright (c) The BitsLab.MoveBit Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::{
    analyzer_handler::*,
    context::*,
    utils::{path_concat, FileRange},
};
use lsp_server::*;
use lsp_types::*;
use move_model::{
    ast::{ExpData::*, Operation::*, SpecBlockTarget},
    model::{FunId, GlobalEnv, ModuleEnv, ModuleId, StructId},
};
use std::{
    collections::BTreeSet,
    ops::Deref,
    path::{Path, PathBuf},
};

/// Handles on_references_request of the language server.
pub fn on_references_request(context: &Context, request: &Request) -> lsp_server::Response {
    log::info!("on_references_request request = {:?}", request);
    let parameters = serde_json::from_value::<ReferenceParams>(request.params.clone())
        .expect("could not deserialize Reference request");
    let fpath = parameters
        .text_document_position
        .text_document
        .uri
        .to_file_path()
        .unwrap();
    let loc = parameters.text_document_position.position;
    let line = loc.line;
    let col = loc.character;
    let fpath = path_concat(std::env::current_dir().unwrap().as_path(), fpath.as_path());

    let mut handler = Handler::new(fpath.clone(), line, col);
    match context.projects.get_project(&fpath) {
        Some(x) => x,
        None => {
            log::error!("project not found:{:?}", fpath.as_path());
            return Response {
                id: "".to_string().into(),
                result: Some(serde_json::json!({"msg": "No available project"})),
                error: None,
            };
        },
    }
    .run_visitor_for_file(&mut handler, &fpath, String::default());
    let locations = handler.convert_to_locations();
    let r = Response::new_ok(
        request.id.clone(),
        serde_json::to_value(GotoDefinitionResponse::Array(locations)).unwrap(),
    );
    let ret_response = r.clone();
    log::info!(
        "------------------------------------\n<on_references_request>ret_response = \n{:?}\n\n",
        ret_response
    );
    context
        .connection
        .sender
        .send(Message::Response(r))
        .unwrap();
    ret_response
}

pub(crate) struct Handler {
    /// The file we are looking for.
    pub(crate) filepath: PathBuf,
    pub(crate) line: u32,
    pub(crate) col: u32,

    pub(crate) mouse_span: codespan::Span,
    pub(crate) capture_items_span: Vec<codespan::Span>,
    pub(crate) result_candidates: Vec<FileRange>,
    pub(crate) result_ref_candidates: Vec<Vec<FileRange>>,
    pub(crate) target_module_id: ModuleId,
}

impl Handler {
    pub(crate) fn new(filepath: impl Into<PathBuf>, line: u32, col: u32) -> Self {
        Self {
            filepath: filepath.into(),
            line,
            col,
            mouse_span: Default::default(),
            capture_items_span: vec![],
            result_candidates: vec![],
            result_ref_candidates: vec![],
            target_module_id: ModuleId::new(0),
        }
    }

    fn convert_to_locations(&mut self) -> Vec<Location> {
        let mut most_clost_item_idx: usize = 0;
        if let Some(item_idx) = find_smallest_length_index(&self.capture_items_span) {
            most_clost_item_idx = item_idx;
        }

        if !self.result_ref_candidates.is_empty()
            && most_clost_item_idx < self.result_ref_candidates.len()
        {
            let mut ret = Vec::with_capacity(self.result_ref_candidates.len());
            for item in &self.result_ref_candidates[most_clost_item_idx] {
                ret.push(item.mk_location());
            }
            self.capture_items_span.clear();
            self.result_candidates.clear();
            self.result_ref_candidates.clear();
            return ret;
        }
        self.capture_items_span.clear();
        self.result_candidates.clear();
        self.result_ref_candidates.clear();
        vec![]
    }

    fn get_mouse_loc(&mut self, env: &GlobalEnv, target_fn_or_struct_loc: &move_model::model::Loc) {
        let mut mouse_line_first_col = move_model::model::Loc::new(
            target_fn_or_struct_loc.file_id(),
            codespan::Span::new(
                target_fn_or_struct_loc.span().start() + codespan::ByteOffset(1),
                target_fn_or_struct_loc.span().start() + codespan::ByteOffset(2),
            ),
        );
        let mut mouse_loc = env.get_location(&mouse_line_first_col).unwrap();
        // locate to self.line first column
        while mouse_loc.line.0 < self.line {
            mouse_line_first_col = move_model::model::Loc::new(
                target_fn_or_struct_loc.file_id(),
                codespan::Span::new(
                    mouse_line_first_col.span().start() + codespan::ByteOffset(1),
                    target_fn_or_struct_loc.span().end(),
                ),
            );
            mouse_loc = env.get_location(&mouse_line_first_col).unwrap();
        }
        // locate to self.line last column
        let mut mouse_line_last_col = move_model::model::Loc::new(
            target_fn_or_struct_loc.file_id(),
            codespan::Span::new(
                mouse_line_first_col.span().start() + codespan::ByteOffset(1),
                mouse_line_first_col.span().start() + codespan::ByteOffset(2),
            ),
        );

        mouse_loc = env.get_location(&mouse_line_last_col).unwrap();
        // locate to self.line first column
        while mouse_loc.column.0 < self.col && mouse_loc.line.0 == self.line {
            mouse_line_last_col = move_model::model::Loc::new(
                target_fn_or_struct_loc.file_id(),
                codespan::Span::new(
                    mouse_line_last_col.span().start() + codespan::ByteOffset(1),
                    target_fn_or_struct_loc.span().end(),
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
        log::info!("<on_references> -- mouse_source = {:?}", mouse_source);

        self.mouse_span = codespan::Span::new(
            mouse_line_first_col.span().start(),
            mouse_line_last_col.span().start(),
        );
    }

    fn get_which_modules_used_target_module(
        &mut self,
        env: &GlobalEnv,
        target_module: &ModuleEnv,
    ) -> BTreeSet<ModuleId> {
        env.get_modules()
            .filter_map(|module_env| {
                let target_module_name_symbol = target_module.get_name().name();
                let target_module_name_dis = target_module_name_symbol.display(env.symbol_pool());
                let target_module_name_str = target_module_name_dis.to_string();

                if let Ok(module_src) = env.get_source(&module_env.get_loc()) {
                    if module_src.contains(target_module_name_str.as_str()) {
                        return Some(module_env.get_id());
                    }
                }
                None
            })
            .collect()
    }

    /*
    fn get_global_env_all_ty_mapping(&mut self, env: &GlobalEnv) {
        log::info!("lll >> <on_references>get_global_env_all_ty_mapping =======================================\n\n");
        for module_env in env.get_modules() {
            let module_id = module_env.get_id();
            for struct_env in module_env.get_structs() {
                let stc_id = struct_env.get_id();
                let stc_symbol = struct_env.get_name();
            }
        }
    }
    */

    fn process_func(&mut self, env: &GlobalEnv) {
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
            if func_start_pos.line.0 < self.line && self.line < func_end_pos.line.0 {
                target_fun_id = fun.get_id();
                found_target_fun = true;
                break;
            }
        }

        if !found_target_fun {
            log::info!("<on_references> -- not in fun!\n");
            return;
        }

        let target_module = env.get_module(self.target_module_id);
        let target_fun = target_module.get_function(target_fun_id);
        let target_fun_loc = target_fun.get_loc();
        self.get_mouse_loc(env, &target_fun_loc);

        if let Some(exp) = target_fun.get_def().deref() {
            self.process_expr(env, exp);
        };
    }

    fn process_spec_func(&mut self, env: &GlobalEnv) {
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
            log::info!("<on_references> -- not in found_target_spec_fun!\n");
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
            log::info!("<on_references> -- not in struct!\n");
            return;
        }

        let target_module = env.get_module(self.target_module_id);
        let target_struct = target_module.get_struct(target_struct_id);
        let target_struct_loc = target_struct.get_loc();
        self.get_mouse_loc(env, &target_struct_loc);

        for field_env in target_struct.get_fields() {
            let field_name = field_env.get_name();
            let field_name_str = field_name.display(env.symbol_pool());
            log::info!("lll >> field_name = {}", field_name_str);
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
                            // todo: should check mouse_last_col between in scope by atomic_field_loc
                            if atomic_field_loc.span().end() < self.mouse_span.end()
                                || atomic_field_loc.span().start() > self.mouse_span.end()
                            {
                                continue;
                            }
                            log::info!("lll >> atomic_field_source = {:?}", atomic_field_source);
                            let field_type = field_env.get_type();
                            self.process_type(env, &atomic_field_loc, &field_type);
                        }
                    }
                }
            }
        }
    }

    fn process_spec_struct(&mut self, env: &GlobalEnv) {
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
            log::info!("<on_references> -- not in found_target_spec_stct!\n");
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
        exp.visit_post_order(&mut |e| match e {
            Call(..) => {
                self.process_call(env, e);
                true
            },
            _ => true,
        });
    }

    fn process_call(&mut self, env: &GlobalEnv, expdata: &move_model::ast::ExpData) {
        if let Call(node_id, MoveFunction(mid, fid), _) = expdata {
            let this_call_loc = env.get_node_loc(*node_id);
            if this_call_loc.span().start() < self.mouse_span.end()
                && self.mouse_span.end() < this_call_loc.span().end()
            {
                let called_module = env.get_module(*mid);
                let called_fun = called_module.get_function(*fid);
                log::info!(
                    "<on_references> -- process_call -- get_called_functions = {:?}",
                    called_fun.get_full_name_str()
                );

                if let Some(calling_fns) = called_fun.get_calling_functions() {
                    let mut result_candidates: Vec<FileRange> = Vec::new();
                    for caller in calling_fns {
                        let f = env.get_function(caller);
                        let mut caller_fun_loc = f.get_loc();
                        // need locate called_fun.get_full_name_str() in f's body source
                        let f_source = env.get_source(&caller_fun_loc);
                        if let Ok(f_source_str) = f_source {
                            if let Some(index) =
                                f_source_str.find(called_fun.get_name_str().as_str())
                            {
                                let target_len: usize = called_fun.get_name_str().len();
                                let start = caller_fun_loc.span().start()
                                    + codespan::ByteOffset(index.try_into().unwrap());
                                let end =
                                    start + codespan::ByteOffset((target_len).try_into().unwrap());
                                caller_fun_loc = move_model::model::Loc::new(
                                    caller_fun_loc.file_id(),
                                    codespan::Span::new(start, end),
                                );
                            }
                        }
                        let (caller_fun_file, caller_fun_line) =
                            env.get_file_and_location(&caller_fun_loc).unwrap();

                        let result = FileRange {
                            path: PathBuf::from(caller_fun_file),
                            line_start: caller_fun_line.line.0,
                            col_start: caller_fun_line.column.0,
                            line_end: caller_fun_line.line.0,
                            col_end: caller_fun_line.column.0
                                + called_fun.get_name_str().len() as u32,
                        };
                        result_candidates.push(result);
                    }
                    self.result_ref_candidates.push(result_candidates);
                    self.capture_items_span.push(this_call_loc.span());
                }
            }
        }
        if let Call(node_id, SpecFunction(mid, fid, _), _) = expdata {
            let this_call_loc = env.get_node_loc(*node_id);
            if this_call_loc.span().start() < self.mouse_span.end()
                && self.mouse_span.end() < this_call_loc.span().end()
            {
                let called_module = env.get_module(*mid);
                let spec_fun = called_module.get_spec_fun(*fid);
                log::info!(
                    "<on_references> -- process_call -- get_spec_functions = {}",
                    spec_fun.name.display(env.symbol_pool())
                );
                let mut result_candidates: Vec<FileRange> = Vec::new();
                for callee in spec_fun.callees.clone() {
                    let module = env.get_module(callee.module_id);
                    let decl = module.get_spec_fun(callee.id);

                    let (caller_fun_file, caller_fun_line) =
                        env.get_file_and_location(&decl.loc).unwrap();
                    let result = FileRange {
                        path: PathBuf::from(caller_fun_file),
                        line_start: caller_fun_line.line.0,
                        col_start: caller_fun_line.column.0,
                        line_end: caller_fun_line.line.0,
                        col_end: caller_fun_line.column.0,
                    };
                    result_candidates.push(result);
                }
                self.result_ref_candidates.push(result_candidates);
                self.capture_items_span.push(this_call_loc.span());
            }
        }
    }

    fn process_type(
        &mut self,
        env: &GlobalEnv,
        capture_items_loc: &move_model::model::Loc,
        ty: &move_model::ty::Type,
    ) {
        use move_model::ty::Type::*;

        if let Struct(mid, stid, _) = ty {
            let stc_def_module = env.get_module(*mid);
            let type_struct = stc_def_module.get_struct(*stid);
            let mouse_capture_ty_symbol = type_struct.get_name();
            let mouse_capture_ty_symbol_dis = mouse_capture_ty_symbol.display(env.symbol_pool());
            let mouse_capture_ty_symbol_str = mouse_capture_ty_symbol_dis.to_string();

            let mut result_candidates: Vec<FileRange> = Vec::new();
            for reference_module_id in
                self.get_which_modules_used_target_module(env, &stc_def_module)
            {
                let stc_ref_module = env.get_module(reference_module_id);
                for stc_ref_fn in stc_ref_module.get_functions() {
                    let mut stc_ref_fn_loc = stc_ref_fn.get_loc();
                    while let Ok(stc_ref_fn_source) = env.get_source(&stc_ref_fn_loc) {
                        if let Some(index) =
                            stc_ref_fn_source.find(mouse_capture_ty_symbol_str.as_str())
                        {
                            let capture_ref_ty_start = stc_ref_fn_loc.span().start()
                                + codespan::ByteOffset(index.try_into().unwrap());
                            let capture_ref_ty_end = capture_ref_ty_start
                                + codespan::ByteOffset(
                                    (mouse_capture_ty_symbol_str.len()).try_into().unwrap(),
                                );

                            let result_loc = move_model::model::Loc::new(
                                stc_ref_fn_loc.file_id(),
                                codespan::Span::new(capture_ref_ty_start, capture_ref_ty_end),
                            );

                            let (ref_ty_file, ref_ty_pos) =
                                env.get_file_and_location(&result_loc).unwrap();
                            let result = FileRange {
                                path: PathBuf::from(ref_ty_file).clone(),
                                line_start: ref_ty_pos.line.0,
                                col_start: ref_ty_pos.column.0,
                                line_end: ref_ty_pos.line.0,
                                col_end: ref_ty_pos.column.0
                                    + mouse_capture_ty_symbol_str.len() as u32,
                            };
                            result_candidates.push(result);

                            stc_ref_fn_loc = move_model::model::Loc::new(
                                stc_ref_fn_loc.file_id(),
                                codespan::Span::new(
                                    capture_ref_ty_end,
                                    stc_ref_fn_loc.span().end(),
                                ),
                            );
                        } else {
                            break;
                        }
                    }
                }
            }
            self.result_ref_candidates.push(result_candidates);
            self.capture_items_span.push(capture_items_loc.span());
        }
    }

    fn run_move_model_visitor_internal(&mut self, env: &GlobalEnv, move_file_path: &Path) {
        let candidate_modules =
            crate::utils::get_modules_by_fpath_in_all_modules(env, &PathBuf::from(move_file_path));
        if candidate_modules.is_empty() {
            log::info!("<on_references>cannot get target module\n");
            return;
        }
        for module_env in candidate_modules.iter() {
            self.target_module_id = module_env.get_id();
            if let Some(s) = move_file_path.to_str() {
                if s.contains(".spec") {
                    self.process_spec_func(env);
                    self.process_spec_struct(env);
                } else {
                    self.process_func(env);
                    self.process_struct(env);
                }
            }
        }
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
            "reference, file:{:?} line:{} col:{}",
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
