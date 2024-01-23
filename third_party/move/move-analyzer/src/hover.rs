// Copyright (c) The BitsLab.MoveBit Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::{analyzer_handler::*, context::*, utils::path_concat};
use lsp_server::*;
use lsp_types::*;
use move_model::{
    ast::{ExpData::*, Operation::*, SpecBlockTarget},
    model::{FunId, GlobalEnv, ModuleId, StructId},
};
use std::{
    ops::Deref,
    path::{Path, PathBuf},
};

/// Handles on_hover_request of the language server.
pub fn on_hover_request(context: &Context, request: &Request) -> lsp_server::Response {
    log::info!("on_hover_request request = {:?}", request);
    let parameters = serde_json::from_value::<HoverParams>(request.params.clone())
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

    let r = Response::new_ok(
        request.id.clone(),
        serde_json::to_value(handler.get_result()).unwrap(),
    );
    let ret_response = r.clone();
    log::info!(
        "------------------------------------\n<on_hover>ret_response = \n{:?}\n\n",
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
    pub(crate) result_candidates: Vec<String>,
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
            target_module_id: ModuleId::new(0),
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

    fn get_result(&mut self) -> Hover {
        let mut most_clost_item_idx: usize = 0;
        if let Some(item_idx) = find_smallest_length_index(&self.capture_items_span) {
            most_clost_item_idx = item_idx;
        }

        if !self.result_candidates.is_empty() && most_clost_item_idx < self.result_candidates.len()
        {
            let ret_str = self.result_candidates[most_clost_item_idx].clone();
            self.capture_items_span.clear();
            self.result_candidates.clear();
            let hover = Hover {
                contents: HoverContents::Scalar(MarkedString::String(ret_str)),
                range: None,
            };
            return hover;
        }
        self.capture_items_span.clear();
        self.result_candidates.clear();

        Hover {
            contents: HoverContents::Scalar(MarkedString::String("null".to_string())),
            range: None,
        }
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
        log::info!("<on hover> mouse_source = {:?}", mouse_source);

        self.mouse_span = codespan::Span::new(
            mouse_line_first_col.span().start(),
            mouse_line_last_col.span().start(),
        );
    }

    fn process_use_decl(&mut self, env: &GlobalEnv) {
        let mut target_module = env.get_module(self.target_module_id);
        let spool = env.symbol_pool();
        let mut ref_module = String::default();
        let mut target_stct_or_fn = String::default();
        let mut found_target_stct_or_fn = false;
        let mut capture_items_loc = move_model::model::Loc::default();
        for use_decl in target_module.get_use_decls() {
            if let Some(use_pos) = env.get_location(&use_decl.loc) {
                if u32::from(use_pos.line) != self.line {
                    continue;
                }
            }
            log::trace!(
                "use_decl.loc = {:?}, use_decl.loc.len = {:?}",
                use_decl.loc,
                use_decl.loc.span().end() - use_decl.loc.span().start()
            );

            if !use_decl.members.is_empty() {
                for (member_loc, name, _) in use_decl.members.clone().into_iter() {
                    log::trace!("member_loc = {:?} ---", env.get_location(&member_loc));
                    if self.check_move_model_loc_contains_mouse_pos(env, &member_loc) {
                        log::trace!("find use symbol = {}", name.display(spool));
                        target_stct_or_fn = name.display(spool).to_string();
                        found_target_stct_or_fn = true;
                        ref_module = use_decl.module_name.display_full(env).to_string();
                        capture_items_loc = member_loc;
                        break;
                    }
                }
            }
            if found_target_stct_or_fn {
                break;
            }
        }

        if !found_target_stct_or_fn {
            log::info!("<on hover> not found_target_stct_or_fn in use_decl");
            return;
        }

        for module in env.get_modules() {
            let module_name = module.get_name().display(env).to_string();
            if ref_module.contains(&module_name) {
                target_module = module;
                break;
            }
        }
        for stct in target_module.get_structs() {
            if stct.get_full_name_str().contains(&target_stct_or_fn) {
                self.capture_items_span.push(capture_items_loc.span());
                self.result_candidates.push(stct.get_full_name_str());
                return;
            }
        }
        for func in target_module.get_functions() {
            if func.get_name_str().contains(&target_stct_or_fn) {
                log::info!("<on hover> func.get_name_str() = {:?}", func.get_name_str());
                self.capture_items_span.push(capture_items_loc.span());
                self.result_candidates.push(func.get_header_string());
                return;
            }
        }
    }

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
            log::warn!("<on hover> not found_target_fun");
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
            log::warn!("<on hover> not found_target_spec_fun");
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
            log::warn!("<on hover> not found_target_struct");
            return;
        }

        let target_module = env.get_module(self.target_module_id);
        let target_struct = target_module.get_struct(target_struct_id);
        let target_struct_loc = target_struct.get_loc();
        self.get_mouse_loc(env, &target_struct_loc);

        for field_env in target_struct.get_fields() {
            let field_name = field_env.get_name();
            let field_name_str = field_name.display(env.symbol_pool());
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

                            if atomic_field_loc.span().end() < self.mouse_span.end()
                                || atomic_field_loc.span().start() > self.mouse_span.end()
                            {
                                continue;
                            }
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
            log::warn!("<on hover> not found_target_spec_stct");
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
        exp.visit_post_order(&mut |e| {
            match e {
                Value(node_id, _v) => {
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
                            let named_const_str = named_const.get_name().display(spool).to_string();
                            if value_str.contains(&named_const_str) {
                                self.capture_items_span.push(value_loc.span());
                                self.result_candidates
                                    .push(env.display(&named_const.get_value()).to_string());
                            }
                        }
                    }
                    true
                },
                Call(..) => {
                    self.process_call(env, e);
                    true
                },
                LocalVar(node_id, localvar_symbol) => {
                    let localvar_loc = env.get_node_loc(*node_id);

                    if localvar_loc.span().start() > self.mouse_span.end()
                        || self.mouse_span.end() > localvar_loc.span().end()
                    {
                        return true;
                    }

                    // The variable names "__update_iter_flag" and "__upper_bound_value" seem to be special,
                    // causing hover errors in a "for (i 1..10)" loop. Handling this issue accordingly.
                    if localvar_symbol.display(env.symbol_pool()).to_string()
                        == "__update_iter_flag"
                        || localvar_symbol.display(env.symbol_pool()).to_string()
                            == "__upper_bound_value"
                    {
                        return true;
                    }

                    log::trace!(
                        "local val symbol: {}",
                        localvar_symbol.display(env.symbol_pool()).to_string()
                    );

                    if let Some(node_type) = env.get_node_type_opt(*node_id) {
                        self.process_type(env, &localvar_loc, &node_type);
                    }
                    true
                },
                Temporary(node_id, _) => {
                    let tmpvar_loc = env.get_node_loc(*node_id);
                    if tmpvar_loc.span().start() > self.mouse_span.end()
                        || self.mouse_span.end() > tmpvar_loc.span().end()
                    {
                        return true;
                    }

                    if let Some(node_type) = env.get_node_type_opt(*node_id) {
                        self.process_type(env, &tmpvar_loc, &node_type);
                    }
                    true
                },
                Block(_, pattern, _, _) => {
                    for (sym_node_id, _) in pattern.vars() {
                        let sym_loc = env.get_node_loc(sym_node_id);
                        if sym_loc.span().start() > self.mouse_span.end()
                            || self.mouse_span.end() > sym_loc.span().end()
                        {
                            return true;
                        }

                        if let Some(node_type) = env.get_node_type_opt(sym_node_id) {
                            self.process_type(env, &sym_loc, &node_type);
                        }
                    }
                    true
                },
                _ => true,
            }
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
                self.capture_items_span.push(this_call_loc.span());
                self.result_candidates.push(called_fun.get_header_string());
            }
        }

        if let Call(node_id, Select(mid, sid, fid), _) = expdata {
            let this_call_loc = env.get_node_loc(*node_id);
            if this_call_loc.span().start() > self.mouse_span.end()
                || self.mouse_span.end() > this_call_loc.span().end()
            {
                return;
            }

            let called_module = env.get_module(*mid);
            let called_struct = called_module.get_struct(*sid);
            let called_field = called_struct.get_field(*fid);
            let field_type = called_field.get_type();
            self.process_type(env, &this_call_loc, &field_type);
        }

        if let Call(node_id, SpecFunction(mid, fid, _), _) = expdata {
            let this_call_loc = env.get_node_loc(*node_id);
            if this_call_loc.span().start() < self.mouse_span.end()
                && self.mouse_span.end() < this_call_loc.span().end()
            {
                let called_module = env.get_module(*mid);
                let spec_fun = called_module.get_spec_fun(*fid);
                self.capture_items_span.push(this_call_loc.span());
                self.result_candidates
                    .push(spec_fun.name.display(env.symbol_pool()).to_string());

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

        if let Call(node_id, Pack(_, _), _) = expdata {
            let op_loc = env.get_node_loc(*node_id);
            if op_loc.span().start() < self.mouse_span.end()
                && self.mouse_span.end() < op_loc.span().end()
            {
                if let Some(node_type) = env.get_node_type_opt(*node_id) {
                    self.process_type(env, &op_loc, &node_type);
                }
            }
        }
    }

    fn process_type(
        &mut self,
        env: &GlobalEnv,
        capture_items_loc: &move_model::model::Loc,
        ty: &move_model::ty::Type,
    ) {
        use move_model::ty::TypeDisplayContext;
        let display_context = TypeDisplayContext::new(env);
        let type_display = ty.display(&display_context);
        self.capture_items_span.push((*capture_items_loc).span());
        self.result_candidates.push(type_display.to_string());
    }

    fn run_move_model_visitor_internal(&mut self, env: &GlobalEnv, move_file_path: &Path) {
        let candidate_modules =
            crate::utils::get_modules_by_fpath_in_all_modules(env, &PathBuf::from(move_file_path));
        if candidate_modules.is_empty() {
            log::info!("<on hover>cannot get target module\n");
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
            "hover,file:{:?} line:{} col:{}",
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
