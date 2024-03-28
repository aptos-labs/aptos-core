// Copyright (c) The BitsLab.MoveBit Contributors
// SPDX-License-Identifier: Apache-2.0

#[allow(unused_imports)]
#[macro_use(sp)]
extern crate move_ir_types;

#[macro_export]
macro_rules! impl_convert_loc {
    ($struct_name:ident) => {
        impl ConvertLoc for $struct_name {
            fn convert_file_hash_filepath(&self, hash: &FileHash) -> Option<PathBuf> {
                self.hash_file
                    .as_ref()
                    .borrow()
                    .get_path(hash)
                    .map(|x| x.clone())
            }

            fn convert_loc_range(&self, loc: &Loc) -> Option<FileRange> {
                self.convert_file_hash_filepath(&loc.file_hash())
                    .map(|file| {
                        self.file_line_mapping.as_ref().borrow().translate(
                            &file,
                            loc.start(),
                            loc.end(),
                        )
                    })
                    .flatten()
            }
        }
    };
}

pub mod analyzer_handler;
pub mod completion;
pub mod context;
pub mod diagnostics;
pub mod goto_definition;
pub mod hover;
pub mod inlay_hints;
pub mod item;
pub mod multiproject;
pub mod project;
pub mod project_manager;
pub mod references;
pub mod utils;

pub mod move_generate_spec;
pub mod move_generate_spec_file;
pub mod move_generate_spec_sel;
pub mod move_generate_spec_utils;
pub mod symbols;
pub mod type_display_for_spec;
