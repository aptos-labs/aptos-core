// Copyright (c) The BitsLab.MoveBit Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::{analyzer_handler::*, context::*, utils::*};
use lsp_server::*;
use lsp_types::*;
use move_model::{
    ast::{ExpData::*, Operation::*, SpecBlockTarget},
    model::{FunctionEnv, GlobalEnv, ModuleId},
};
use std::{
    ops::Deref,
    path::{Path, PathBuf},
};

/// Handles inlay_hints request of the language server.
pub fn on_inlay_hints(
    context: &Context,
    request: &Request,
    inlay_hints_config: &InlayHintsConfig,
) -> lsp_server::Response {
    log::info!("on_inlay_hints request = {:?}", request);
    let parameters = serde_json::from_value::<InlayHintParams>(request.params.clone())
        .expect("could not deserialize go-to-def request");
    let fpath = parameters.text_document.uri.to_file_path().unwrap();
    let fpath = path_concat(std::env::current_dir().unwrap().as_path(), fpath.as_path());

    if !inlay_hints_config.enable {
        log::info!("inlay-hints is not enabled.");
        return Response {
            id: "".to_string().into(),
            result: Some(serde_json::json!({"msg": "Inlay-hints is not enabled."})),
            error: None,
        };
    }

    let mut handler = Handler::new(fpath.clone(), parameters.range);
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

    let hints = Some(handler.reuslts);
    let r = Response::new_ok(request.id.clone(), serde_json::to_value(hints).unwrap());
    let ret_response = r.clone();
    context
        .connection
        .sender
        .send(Message::Response(r))
        .unwrap();
    ret_response
}

#[derive(Clone, Copy, serde::Deserialize, Debug)]
pub struct InlayHintsConfig {
    pub enable: bool,
}

impl Default for InlayHintsConfig {
    fn default() -> Self {
        Self { enable: true }
    }
}

struct Handler {
    range: FileRange,
    reuslts: Vec<InlayHint>,
    target_module_id: ModuleId,
}

impl Handler {
    fn new(fpath: PathBuf, range: lsp_types::Range) -> Self {
        Self {
            range: FileRange {
                path: fpath,
                line_start: range.start.line,
                col_start: range.end.character,
                line_end: range.end.line,
                col_end: range.end.character + 1,
            },
            reuslts: Default::default(),
            target_module_id: ModuleId::new(0),
        }
    }

    fn process_func(&mut self, env: &GlobalEnv) {
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

            if self.range.line_start <= func_start_pos.line.0
                && func_end_pos.line.0 <= self.range.line_end
            {
                if let Some(exp) = fun.get_def().deref() {
                    log::trace!("process funcation: {}", fun.get_name_string());
                    self.process_expr(env, &fun, exp);
                }
            }
        }
    }

    fn process_spec_func(&mut self, env: &GlobalEnv) {
        let target_module = env.get_module(self.target_module_id);
        for spec_block_info in target_module.get_spec_block_infos() {
            if let SpecBlockTarget::Function(_, fun_id) = spec_block_info.target {
                let (_, block_start_pos) = env.get_file_and_location(&spec_block_info.loc).unwrap();
                let (_, block_end_pos) = env
                    .get_file_and_location(&move_model::model::Loc::new(
                        spec_block_info.loc.file_id(),
                        codespan::Span::new(
                            spec_block_info.loc.span().end(),
                            spec_block_info.loc.span().end(),
                        ),
                    ))
                    .unwrap();
                if self.range.line_start <= block_start_pos.line.0
                    && block_end_pos.line.0 <= self.range.line_end
                {
                    let target_fn = target_module.get_function(fun_id);
                    let target_fn_spec = target_fn.get_spec();
                    log::info!(
                        "<inlay_hing> -- target_fun's spec = {}",
                        env.display(&*target_fn_spec)
                    );
                    for cond in target_fn_spec.conditions.clone() {
                        for exp in cond.all_exps() {
                            self.process_expr(env, &target_fn, exp);
                        }
                    }
                }
            }

            if let SpecBlockTarget::FunctionCode(_, fun_id, _) = spec_block_info.target {
                let (_, block_start_pos) = env.get_file_and_location(&spec_block_info.loc).unwrap();
                let (_, block_end_pos) = env
                    .get_file_and_location(&move_model::model::Loc::new(
                        spec_block_info.loc.file_id(),
                        codespan::Span::new(
                            spec_block_info.loc.span().end(),
                            spec_block_info.loc.span().end(),
                        ),
                    ))
                    .unwrap();
                if self.range.line_start <= block_start_pos.line.0
                    && block_end_pos.line.0 <= self.range.line_end
                {
                    let target_fn = target_module.get_function(fun_id);
                    let target_fn_spec = target_fn.get_spec();
                    for cond in target_fn_spec.conditions.clone() {
                        for exp in cond.all_exps() {
                            self.process_expr(env, &target_fn, exp);
                        }
                    }
                }
            }

            if let SpecBlockTarget::Schema(_, _, _) = spec_block_info.target {
                let (_, block_start_pos) = env.get_file_and_location(&spec_block_info.loc).unwrap();
                let (_, block_end_pos) = env
                    .get_file_and_location(&move_model::model::Loc::new(
                        spec_block_info.loc.file_id(),
                        codespan::Span::new(
                            spec_block_info.loc.span().end(),
                            spec_block_info.loc.span().end(),
                        ),
                    ))
                    .unwrap();
                if self.range.line_start <= block_start_pos.line.0
                    && block_end_pos.line.0 <= self.range.line_end
                {
                    log::info!(
                        "<inlay_hint> -- SpecBlockTarget::Schema, spec_block_info.loc = {:?}",
                        env.get_file_and_location(&spec_block_info.loc.clone())
                    );
                }
            }
        }
    }

    fn process_expr(&mut self, env: &GlobalEnv, fun: &FunctionEnv, exp: &move_model::ast::Exp) {
        let mut is_visited = vec![];
        exp.visit_post_order(&mut |e| match e {
            Call(..) => {
                self.process_call(env, e);
                true
            },
            LocalVar(node_id, localvar_symbol) => {
                let mut localvar_loc = env.get_node_loc(*node_id);
                localvar_loc = move_model::model::Loc::new(
                    localvar_loc.file_id(),
                    codespan::Span::new(
                        localvar_loc.span().start(),
                        localvar_loc.span().end() + codespan::ByteOffset((2).try_into().unwrap()),
                    ),
                );
                if localvar_symbol.display(env.symbol_pool()).to_string() == "__update_iter_flag"
                    || localvar_symbol.display(env.symbol_pool()).to_string()
                        == "__upper_bound_value"
                {
                    return true;
                }

                if is_visited.contains(&localvar_loc.span().start().0) {
                    return true;
                }
                is_visited.push(localvar_loc.span().start().0);

                log::trace!(
                    "lll >> exp.visit localvar_loc = {:?}",
                    env.get_location(&localvar_loc)
                );
                log::trace!(
                    "lll >> exp.visit localvar_symbol = {}",
                    localvar_symbol.display(env.symbol_pool()).to_string()
                );
                if let Some(node_type) = env.get_node_type_opt(*node_id) {
                    if let Ok(local_var_str) = env.get_source(&localvar_loc) {
                        if let Some(index) = local_var_str.find(
                            localvar_symbol
                                .display(env.symbol_pool())
                                .to_string()
                                .as_str(),
                        ) {
                            let inlay_hint_pos = move_model::model::Loc::new(
                                localvar_loc.file_id(),
                                codespan::Span::new(
                                    localvar_loc.span().start()
                                        + codespan::ByteOffset(
                                            (index
                                                + localvar_symbol
                                                    .display(env.symbol_pool())
                                                    .to_string()
                                                    .len())
                                            .try_into()
                                            .unwrap(),
                                        ),
                                    localvar_loc.span().end(),
                                ),
                            );
                            let next_char = local_var_str.chars().nth(
                                index
                                    + localvar_symbol.display(env.symbol_pool()).to_string().len(),
                            );
                            match next_char {
                                Some('.') => return true,
                                Some(':') => return true,
                                _ => {
                                    log::trace!(
                                        "local_var_str[{:?}] inlay_hint_pos = {:?}",
                                        local_var_str,
                                        inlay_hint_pos
                                    );
                                    self.process_type(env, &inlay_hint_pos, &node_type);
                                },
                            }
                        }
                    }
                }
                true
            },
            Temporary(node_id, idx) => {
                let mut tmpvar_loc = env.get_node_loc(*node_id);
                tmpvar_loc = move_model::model::Loc::new(
                    tmpvar_loc.file_id(),
                    codespan::Span::new(
                        tmpvar_loc.span().start(),
                        tmpvar_loc.span().end() + codespan::ByteOffset((2).try_into().unwrap()),
                    ),
                );

                let mut tmp_var_name_str = "".to_string();
                if let Some(name) = fun.get_parameters().get(*idx).map(|p| p.0) {
                    tmp_var_name_str = name.display(env.symbol_pool()).to_string();
                }
                if let Some(node_type) = env.get_node_type_opt(*node_id) {
                    if let Ok(tmp_var_str) = env.get_source(&tmpvar_loc) {
                        if let Some(index) = tmp_var_str.find(tmp_var_name_str.as_str()) {
                            let inlay_hint_pos = move_model::model::Loc::new(
                                tmpvar_loc.file_id(),
                                codespan::Span::new(
                                    tmpvar_loc.span().start()
                                        + codespan::ByteOffset(
                                            (index + tmp_var_name_str.len()).try_into().unwrap(),
                                        ),
                                    tmpvar_loc.span().end(),
                                ),
                            );
                            let next_char = tmp_var_str.chars().nth(index + tmp_var_name_str.len());
                            match next_char {
                                Some('.') => return true,
                                Some(':') => return true,
                                _ => {
                                    self.process_type(env, &inlay_hint_pos, &node_type);
                                },
                            }
                        }
                    }
                }
                true
            },
            Block(_, pattern, _, _) => {
                for (sym_node_id, sym_symbol) in pattern.vars() {
                    let mut sym_loc = env.get_node_loc(sym_node_id);
                    sym_loc = move_model::model::Loc::new(
                        sym_loc.file_id(),
                        codespan::Span::new(
                            sym_loc.span().start(),
                            sym_loc.span().end() + codespan::ByteOffset((2).try_into().unwrap()),
                        ),
                    );

                    if let Some(node_type) = env.get_node_type_opt(sym_node_id) {
                        if let Ok(pattern_sym_str) = env.get_source(&sym_loc) {
                            if let Some(index) = pattern_sym_str
                                .find(sym_symbol.display(env.symbol_pool()).to_string().as_str())
                            {
                                let inlay_hint_pos = move_model::model::Loc::new(
                                    sym_loc.file_id(),
                                    codespan::Span::new(
                                        sym_loc.span().start()
                                            + codespan::ByteOffset(
                                                (index
                                                    + sym_symbol
                                                        .display(env.symbol_pool())
                                                        .to_string()
                                                        .len())
                                                .try_into()
                                                .unwrap(),
                                            ),
                                        sym_loc.span().end(),
                                    ),
                                );
                                let next_char = pattern_sym_str.chars().nth(
                                    index + sym_symbol.display(env.symbol_pool()).to_string().len(),
                                );
                                match next_char {
                                    Some('.') => return true,
                                    Some(':') => return true,
                                    _ => {
                                        log::trace!(
                                            "pattern sym [{:?}] inlay_hint_pos = {:?}",
                                            pattern_sym_str,
                                            inlay_hint_pos
                                        );
                                        self.process_type(env, &inlay_hint_pos, &node_type);
                                    },
                                }
                            }
                        }
                    }
                }
                true
            },
            _ => true,
        });
    }

    fn process_call(&mut self, env: &GlobalEnv, expdata: &move_model::ast::ExpData) {
        if let Call(node_id, Select(mid, sid, fid), _) = expdata {
            let this_call_loc = env.get_node_loc(*node_id);
            let called_module = env.get_module(*mid);
            let called_struct = called_module.get_struct(*sid);
            let called_field = called_struct.get_field(*fid);
            let field_type = called_field.get_type();

            if let Ok(link_access_source) = env.get_source(&this_call_loc) {
                let called_field_name = ".".to_string()
                    + called_field
                        .get_name()
                        .display(env.symbol_pool())
                        .to_string()
                        .as_str();

                if let Some(index) = link_access_source.find(called_field_name.as_str()) {
                    let dis = index + called_field_name.len();
                    let inlay_hint_pos = move_model::model::Loc::new(
                        this_call_loc.file_id(),
                        codespan::Span::new(
                            this_call_loc.span().start()
                                + codespan::ByteOffset((dis).try_into().unwrap()),
                            this_call_loc.span().end(),
                        ),
                    );
                    self.process_type(env, &inlay_hint_pos, &field_type);
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
        let (capture_items_file, capture_items_pos) =
            env.get_file_and_location(capture_items_loc).unwrap();

        let mut definition_pos = FileRange {
            path: PathBuf::from(capture_items_file),
            line_start: 0,
            col_start: 0,
            line_end: 0,
            col_end: 0,
        };

        let mut unpack_ty = ty.clone();
        match unpack_ty {
            move_model::ty::Type::Reference(_, _) => {
                unpack_ty = ty.skip_reference().clone();
            },
            move_model::ty::Type::Vector(_) => {
                unpack_ty = ty.get_vector_element_type().unwrap();
            },
            _ => {},
        }

        if let move_model::ty::Type::Struct(mid, sid, _) = unpack_ty {
            let module_env = env.get_module(mid);
            let struct_env = module_env.get_struct(sid);
            let struct_loc = struct_env.get_loc();
            if let Some((def_file, def_loc)) = env.get_file_and_location(&struct_loc) {
                definition_pos.path = PathBuf::from(def_file);
                definition_pos.line_start = def_loc.line.0;
                definition_pos.col_start = def_loc.column.0;
                definition_pos.line_end = def_loc.line.0;
                definition_pos.col_end = def_loc.column.0;
            }
        }

        self.reuslts.push(mk_inlay_hits(
            Position {
                line: capture_items_pos.line.0,
                character: capture_items_pos.column.0,
            },
            para_inlay_hints_parts(&type_display.to_string(), definition_pos),
            InlayHintKind::TYPE,
        ));
    }

    fn run_move_model_visitor_internal(&mut self, env: &GlobalEnv, move_file_path: &Path) {
        let candidate_modules =
            crate::utils::get_modules_by_fpath_in_all_modules(env, &PathBuf::from(move_file_path));
        if candidate_modules.is_empty() {
            log::info!("<inlay_hints>cannot get target module\n");
            return;
        }
        for module_env in candidate_modules.iter() {
            self.target_module_id = module_env.get_id();
            if let Some(s) = move_file_path.to_str() {
                if !s.contains(".spec") {
                    self.process_func(env);
                } else {
                    self.process_spec_func(env);
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
        write!(f, "inlay hints")
    }
}

fn mk_inlay_hits(pos: Position, label: InlayHintLabel, kind: InlayHintKind) -> InlayHint {
    InlayHint {
        position: pos,
        label,
        kind: Some(kind),
        text_edits: None,
        tooltip: None,
        padding_left: Some(true),
        padding_right: Some(true),
        data: None,
    }
}

fn para_inlay_hints_parts(name: &str, label_pos: FileRange) -> InlayHintLabel {
    InlayHintLabel::LabelParts(vec![InlayHintLabelPart {
        value: format!(": {}", name),
        tooltip: None,
        location: None,
        command: mk_command(label_pos),
    }])
}

fn mk_command(label_pos: FileRange) -> Option<Command> {
    Some(MoveAnalyzerClientCommands::GotoDefinition(label_pos.mk_location()).to_lsp_command())
}
