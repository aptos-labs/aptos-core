// Copyright (c) The BitsLab.MoveBit Contributors
// SPDX-License-Identifier: Apache-2.0

//! This file primarily modifies the `TypeDisplay` structure and its `fmt::Display` trait
//! within the `move-model`. The objective is to adjust the `struct_str` method in the
//! `TypeDisplay` structure so that when printing a struct, it only displays `struct_name`
//! for structs within this module. For structs from other modules, it prints in the form
//! "module_name::struct_name" instead of displaying all structs as "module_name::struct_name".

use move_model::{
    ast::ModuleName,
    model::{ModuleEnv, ModuleId, StructId},
    symbol::Symbol,
    ty::{ReferenceKind, Type, TypeDisplayContext},
};
use std::{collections::HashMap, fmt, fmt::Formatter};
pub struct TypeDisplayForSpec<'a> {
    pub type_: &'a Type,
    pub context: &'a TypeDisplayContext<'a>,
    pub using_module_map: &'a HashMap<ModuleName, Vec<Symbol>>,
    pub module_env: &'a ModuleEnv<'a>,
}

impl<'a> TypeDisplayForSpec<'a> {
    fn make(&self, ty: &'a Type) -> TypeDisplayForSpec<'a> {
        TypeDisplayForSpec {
            type_: ty,
            context: self.context,
            module_env: self.module_env,
            using_module_map: self.using_module_map,
        }
    }
}

impl<'a> fmt::Display for TypeDisplayForSpec<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        use Type::*;
        let comma_list = |f: &mut Formatter<'_>, ts: &[Type]| -> fmt::Result {
            let mut first = true;
            for t in ts {
                if first {
                    first = false
                } else {
                    f.write_str(", ")?;
                }
                write!(f, "{}", self.make(t))?;
            }
            Ok(())
        };
        match self.type_ {
            Primitive(p) => write!(f, "{}", p),
            Tuple(ts) => {
                f.write_str("(")?;
                comma_list(f, ts)?;
                f.write_str(")")
            },
            Vector(t) => {
                write!(f, "vector<{}>", self.make(t))
            },
            TypeDomain(t) => write!(f, "domain<{}>", self.make(t)),
            ResourceDomain(mid, sid, inst_opt) => {
                write!(f, "resources<{}", self.struct_str(*mid, *sid))?;
                if let Some(inst) = inst_opt {
                    f.write_str("<")?;
                    comma_list(f, inst)?;
                    f.write_str(">")?;
                }
                f.write_str(">")
            },
            Fun(a, t) => {
                f.write_str("|")?;
                write!(f, "{}", self.make(a))?;
                f.write_str("|")?;
                write!(f, "{}", self.make(t))
            },
            Struct(mid, sid, ts) => {
                write!(f, "{}", self.struct_str(*mid, *sid))?;
                if !ts.is_empty() {
                    f.write_str("<")?;
                    comma_list(f, ts)?;
                    f.write_str(">")?;
                }
                Ok(())
            },
            Reference(kind, t) => {
                f.write_str("&")?;
                let modifier = match kind {
                    ReferenceKind::Immutable => "",
                    ReferenceKind::Mutable => "mut ",
                };
                f.write_str(modifier)?;
                write!(f, "{}", self.make(t))
            },
            TypeParameter(idx) => {
                if let Some(names) = &self.context.type_param_names {
                    let idx = *idx as usize;
                    if idx < names.len() {
                        write!(f, "{}", names[idx].display(self.context.env.symbol_pool()))
                    } else {
                        write!(f, "#{}", idx)
                    }
                } else {
                    write!(f, "#{}", idx)
                }
            },
            Var(_) => {
                write!(f, "")
            },
            Error => f.write_str("*error*"),
        }
    }
}

impl<'a> TypeDisplayForSpec<'a> {
    fn struct_str(&self, mid: ModuleId, sid: StructId) -> String {
        let env = self.context.env;
        if let Some(builder_table) = self.context.builder_struct_table {
            let qsym = builder_table.get(&(mid, sid)).expect("type known");
            qsym.display(self.context.env).to_string()
        } else {
            let struct_module_env = env.get_module(mid);
            let struct_env = struct_module_env.get_struct(sid);

            let struct_module_env_name = struct_module_env.get_name();
            let struct_env_name = struct_env.get_name();

            if struct_module_env.get_id() == self.module_env.get_id() {
                return struct_env_name.display(env.symbol_pool()).to_string();
            }

            if let Some(members) = self.using_module_map.get(struct_module_env_name) {
                let a = members.iter().find(|&&x| x == struct_env_name);
                if let Some(x) = a {
                    return x.display(env.symbol_pool()).to_string();
                }
            }

            format!(
                "{}::{}",
                struct_module_env_name.display(env),
                struct_env_name.display(env.symbol_pool())
            )
        }
    }
}
