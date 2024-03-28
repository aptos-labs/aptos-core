// Copyright (c) The BitsLab.MoveBit Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::{
    move_generate_spec_utils::{BinOPReason, SpecExpItem},
    type_display_for_spec::TypeDisplayForSpec,
};
use move_model::{
    ast::{
        Exp as MoveModelExp, ExpData as MoveModelExpData, ModuleName,
        Operation as MoveModelOperation,
    },
    model::{FunctionEnv, GlobalEnv, ModuleEnv, StructEnv},
    symbol::Symbol,
    ty::{Type as MoveModelType, TypeDisplayContext},
};
use std::{collections::HashMap, ops::Deref};

#[derive(Default)]
pub struct StructSpecGenerator {
    result: String,
}

impl StructSpecGenerator {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    pub(crate) fn get_result_string(self) -> String {
        self.result
    }

    pub(crate) fn generate(&mut self, x: &StructEnv) {
        self.result.push_str(
            format!(
                "{}spec {}",
                indent(1),
                x.get_name().display(x.symbol_pool())
            )
            .as_str(),
        );
        self.result.push_str("{\n");
        self.result.push_str(format!("{}}}\n", indent(1)).as_str())
    }
}

#[derive(Default)]
pub struct FunSpecGenerator {
    result: String,
}

pub fn generate_fun_spec_zx(
    global_env: &GlobalEnv,
    module_env: &ModuleEnv,
    f: &FunctionEnv,
    using_module_map: &HashMap<ModuleName, Vec<Symbol>>,
) -> String {
    let mut g = FunSpecGenerator::new();
    g.generate_zx(global_env, module_env, f, using_module_map);
    g.get_result_string()
}

pub fn genrate_struct_spec(s: &StructEnv) -> String {
    let mut g = StructSpecGenerator::new();
    g.generate(s);
    g.get_result_string()
}

impl FunSpecGenerator {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    pub(crate) fn get_result_string(self) -> String {
        self.result
    }

    pub(crate) fn generate_zx(
        &mut self,
        global_env: &GlobalEnv,
        module_env: &ModuleEnv,
        f: &FunctionEnv,
        using_module_map: &HashMap<ModuleName, Vec<Symbol>>,
    ) {
        let display_context = f.get_type_display_ctx();
        self.result
            .push_str(format!("{}spec {}", indent(1), f.get_name_str()).as_str());

        let generics = if !f.get_type_parameters().is_empty() {
            format!(
                "<{}>",
                f.get_type_parameters()
                    .iter()
                    .map(|p| p.0.display(f.symbol_pool()).to_string())
                    .collect::<Vec<_>>()
                    .join(",")
            )
        } else {
            "".to_owned()
        };
        self.result.push_str(generics.as_str());
        self.result.push('(');

        let para_len = f.get_parameter_count();
        if para_len > 0 {
            for (index, para) in f.get_parameters().iter().enumerate() {
                self.result
                    .push_str(para.0.display(f.symbol_pool()).to_string().as_str());
                self.result.push_str(": ");
                let display_context_para = TypeDisplayForSpec {
                    type_: &para.1,
                    context: &display_context,
                    module_env,
                    using_module_map,
                };
                let para_type_string = display_context_para.to_string();
                self.result.push_str(para_type_string.as_str());
                if (index + 1) < para_len {
                    self.result.push_str(", ");
                }
            }
        }
        self.result.push(')');

        let return_type = f.get_result_type();
        let display_context_return = TypeDisplayForSpec {
            type_: &return_type,
            context: &display_context,
            module_env,
            using_module_map,
        };
        let mut return_type_string = display_context_return.to_string();

        return_type_string.insert_str(0, ": ");
        if let MoveModelType::Tuple(_) = return_type {
            // ": ()" len is 4
            if return_type_string.len() <= 4 {
                return_type_string = String::new();
            }
        }
        self.result.push_str(return_type_string.as_str());
        self.result.push_str("{\n");
        let assert = Self::generate_body_zx(self, f, global_env);
        self.result.push_str(assert.as_str());
        self.result.push_str(format!("{}}}\n", indent(1)).as_str());
    }

    fn generate_body_zx(&self, f: &FunctionEnv, global_env: &GlobalEnv) -> String {
        let mut statements = String::new();
        if let Some(exp) = f.get_def().deref() {
            FunSpecGenerator::try_emit_exp_zx(self, &mut statements, exp, global_env);
        } else {
            log::trace!("body is none");
            return statements;
        }

        statements
    }
}

impl FunSpecGenerator {
    fn try_emit_exp_zx(&self, statements: &mut String, exp: &MoveModelExp, env: &GlobalEnv) {
        let items = FunSpecGenerator::collect_spec_exp_zx(self, exp, env);
        let display_context = &TypeDisplayContext::new(env);
        for item in items.iter() {
            match item {
                SpecExpItem::BinOP {
                    reason,
                    left,
                    right,
                } => {
                    let _left_node_id = left.as_ref().node_id();
                    let _right_node_id = right.as_ref().node_id();
                    let _left_node_loc = env.get_node_loc(_left_node_id);
                    let _right_node_loc = env.get_node_loc(_right_node_id);
                    let _left_node_type = env.get_node_type(_left_node_id);
                    let _right_node_type = env.get_node_type(_right_node_id);

                    let _left_exp_str = match env.get_source(&_left_node_loc) {
                        Ok(x) => x,
                        Err(_) => continue,
                    };

                    let _right_exp_str = match env.get_source(&_right_node_loc) {
                        Ok(x) => x,
                        Err(_) => continue,
                    };

                    if *reason != BinOPReason::DivByZero
                        && !FunSpecGenerator::is_support_exp(left, env)
                    {
                        continue;
                    }

                    if !FunSpecGenerator::is_support_exp(right, env) {
                        continue;
                    }

                    match reason {
                        BinOPReason::OverFlowADD
                        | BinOPReason::OverFlowMUL
                        | BinOPReason::OverFlowSHL => {
                            let over_type = format!(
                                "MAX_{}",
                                _left_node_type
                                    .display(display_context)
                                    .to_string()
                                    .to_uppercase()
                            );
                            let statements_abort_if = format!(
                                "{}aborts_if {} {} {} > {};\n",
                                indent(2),
                                _left_exp_str,
                                match reason {
                                    BinOPReason::OverFlowADD => "+",
                                    BinOPReason::OverFlowMUL => "*",
                                    BinOPReason::OverFlowSHL => "<<",
                                    _ => unreachable!(),
                                },
                                _right_exp_str,
                                over_type
                            );
                            statements.push_str(statements_abort_if.as_str());
                        },
                        BinOPReason::DivByZero => {
                            statements.push_str(
                                format!("{}aborts_if {} == 0;\n", indent(2), _right_exp_str,)
                                    .as_str(),
                            );
                        },
                        BinOPReason::UnderFlow => {
                            statements.push_str(
                                format!(
                                    "{}aborts_if {} - {} < 0;\n",
                                    indent(2),
                                    _left_exp_str,
                                    _right_exp_str,
                                )
                                .as_str(),
                            );
                        },
                    };
                },
                SpecExpItem::MarcoAbort { if_exp, abort_exp } => {
                    if let MoveModelExpData::Call(_, op, _) = if_exp.as_ref() {
                        match op {
                            MoveModelOperation::Eq
                            | MoveModelOperation::Neq
                            | MoveModelOperation::Lt
                            | MoveModelOperation::Gt
                            | MoveModelOperation::Le
                            | MoveModelOperation::Ge => {
                                FunSpecGenerator::handle_binop_exp(
                                    statements, if_exp, op, abort_exp, env,
                                );
                            },
                            MoveModelOperation::MoveFunction(_, _) => {
                                FunSpecGenerator::handle_funcop_exp(
                                    statements, if_exp, abort_exp, env,
                                );
                            },
                            _ => {},
                        }
                    }
                },
                SpecExpItem::PatternLet { left, right } => {
                    let _left_node_id = left.node_id();
                    let _left_node_loc = env.get_node_loc(_left_node_id);
                    let _left_exp_str = match env.get_source(&_left_node_loc) {
                        Ok(x) => x,
                        Err(_) => continue,
                    };

                    statements.push_str(format!("{}let ", indent(2)).as_str());
                    statements.push_str(_left_exp_str);

                    statements.push_str(" = ");
                    let _right_node_id = right.as_ref().node_id();
                    let _right_node_loc = env.get_node_loc(_right_node_id);
                    let _right_exp_str = match env.get_source(&_right_node_loc) {
                        Ok(x) => x,
                        _ => continue,
                    };
                    statements.push_str(_right_exp_str);
                    statements.push_str(";\n");
                },
                SpecExpItem::BorrowGlobalMut { .. } => {},
                SpecExpItem::TypeName { .. } => {},
                SpecExpItem::TypeOf { .. } => {},
            }
        }
    }

    fn handle_binop_exp(
        statements: &mut String,
        if_exp: &MoveModelExp,
        op: &MoveModelOperation,
        abort_exp: &MoveModelExp,
        env: &GlobalEnv,
    ) {
        fn inverse_binop_zx(op: &MoveModelOperation) -> (String, String) {
            match op {
                MoveModelOperation::Eq => (String::from("=="), String::from("!=")),
                MoveModelOperation::Neq => (String::from("!="), String::from("==")),
                MoveModelOperation::Lt => (String::from("<"), String::from(">=")),
                MoveModelOperation::Gt => (String::from(">"), String::from("<=")),
                MoveModelOperation::Le => (String::from("<="), String::from(">")),
                MoveModelOperation::Ge => (String::from(">="), String::from("<")),
                _ => (String::from("_"), String::from("_")),
            }
        }

        let op2 = inverse_binop_zx(op);

        let if_exp_node_id = if_exp.as_ref().node_id();
        let if_exp_node_loc = env.get_node_loc(if_exp_node_id);
        let if_exp_str = match env.get_source(&if_exp_node_loc) {
            Ok(x) => x,
            Err(_) => return,
        };

        #[allow(unused_assignments)]
        let mut if_exp_inverse_str = String::new();
        if if_exp_str.contains(op2.0.as_str()) {
            if_exp_inverse_str = if_exp_str.replace(op2.0.as_str(), op2.1.as_str());
        } else {
            return;
        }

        let abort_exp_node_id = abort_exp.as_ref().node_id();
        let abort_exp_node_loc = env.get_node_loc(abort_exp_node_id);
        let abort_exp_str = match env.get_source(&abort_exp_node_loc) {
            Ok(x) => x,
            Err(_) => return,
        };

        statements.push_str(
            format!(
                "{}aborts_if {} with {};\n",
                indent(2),
                if_exp_inverse_str,
                abort_exp_str,
            )
            .as_str(),
        );
    }

    fn handle_funcop_exp(
        statements: &mut String,
        if_exp: &MoveModelExp,
        abort_exp: &MoveModelExp,
        env: &GlobalEnv,
    ) {
        let if_exp_node_id = if_exp.as_ref().node_id();
        let if_exp_node_loc = env.get_node_loc(if_exp_node_id);
        let if_exp_str = match env.get_source(&if_exp_node_loc) {
            Ok(x) => x,
            Err(_) => return,
        };

        let abort_exp_node_id = abort_exp.as_ref().node_id();
        let abort_exp_node_loc = env.get_node_loc(abort_exp_node_id);
        let abort_exp_str = match env.get_source(&abort_exp_node_loc) {
            Ok(x) => x,
            Err(_) => return,
        };

        statements.push_str(
            format!(
                "{}aborts_if !{} with {};\n",
                indent(2),
                if_exp_str,
                abort_exp_str,
            )
            .as_str(),
        );
    }
}

pub(crate) fn indent(num: usize) -> String {
    "    ".to_string().repeat(num)
}
