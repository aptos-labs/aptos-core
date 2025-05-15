// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::file_format_generator::peephole_optimizer::optimizers::{
    TransformedCodeChunk, WindowOptimizer,
};
use move_binary_format::file_format::{Bytecode, CodeOffset};
use move_model::model::FunctionEnv;

/// An optimizer for inefficient equality comparison.
pub struct InefficientCmps<'gen, 'env> {
    pub fun_env: &'gen FunctionEnv<'env>,
}

impl<'gen, 'env> InefficientCmps<'gen, 'env> {
    // We need at least 3 instructions, respectively corresponding to pushing op1, pushing op2, and Eq/Neq.
    // All patterns detailed above.
    const MIN_WINDOW_SIZE: usize = 3;
    pub fn new(fun_env:  &'gen FunctionEnv<'env>) -> Self {
        Self { fun_env }
    }
}

impl<'gen, 'env> WindowOptimizer for InefficientCmps<'gen, 'env> {

    fn optimize_window(&self, window: &[Bytecode]) -> Option<(TransformedCodeChunk, usize)> {
        use Bytecode::*;

        if window.len() < Self::MIN_WINDOW_SIZE {
            return None;
        }

        let mut cmp = false;
        let mut mimic_stack: Vec<(usize, &Bytecode)> = Vec::new();

        for (index, instr) in window.iter().enumerate(){
             match instr {
                Eq | Neq => {
                    cmp = true;
                    mimic_stack.push((index, instr));
                    break;
                },

                // operations pushing a single item into the stack
                LdU8(_) | LdU16(_) | LdU32(_) | LdU64(_) | LdU128(_) | LdU256(_) | LdConst(_)
                | LdTrue | LdFalse | CopyLoc(_) | MoveLoc(_) | MutBorrowLoc(_)
                | ImmBorrowLoc(_) => {
                    mimic_stack.push((index, instr));
                },
                // operations popping a single item from the stack
                Pop | StLoc(_) => {
                    mimic_stack.pop();
                },
                WriteRef | MoveTo(_) | MoveToGeneric(_) | VecPushBack(_) => {
                    mimic_stack.pop();
                    mimic_stack.pop();
                },
                VecSwap(_) => {
                    mimic_stack.pop();
                    mimic_stack.pop();
                    mimic_stack.pop();
                }
                // operations popping an item and then pushing a new item
                CastU8 | CastU16| CastU32 | CastU64 | CastU128 | CastU256 | ReadRef | FreezeRef | MutBorrowField(_)
                | MutBorrowVariantField(_) | MutBorrowFieldGeneric(_) | MutBorrowVariantFieldGeneric(_)
                | ImmBorrowField(_) | ImmBorrowVariantField(_) | ImmBorrowFieldGeneric(_)
                | ImmBorrowVariantFieldGeneric(_) | MutBorrowGlobal(_) | MutBorrowGlobalGeneric(_)
                | ImmBorrowGlobal(_) | ImmBorrowGlobalGeneric(_) | Exists(_) | ExistsGeneric(_)
                | MoveFrom(_) | MoveFromGeneric(_) | Not | VecLen(_) => {
                    mimic_stack.pop();
                    mimic_stack.push((index, instr));
                },

                Add | Sub | Mul | Mod | Div | BitOr | BitAnd | Xor | Or | And
                | Lt | Gt | Le | Ge | Shl | Shr | VecImmBorrow(_) | VecMutBorrow(_)
                | VecPopBack(_) => {
                    mimic_stack.pop();
                    mimic_stack.pop();
                    mimic_stack.push((index, instr));
                }

                Ret | BrTrue(_) | BrFalse(_) | Branch(_) | Abort => {
                    break;
                },
                // fundamentally, those operations should be analyzed to
                //  understand how many items are poppoed and how many are pushed.
                // Yet, we simplify the situation by clearing up the mimic stack
                Call(_) | CallGeneric(_) | Pack(_) | PackGeneric(_) | PackVariant(_) | PackVariantGeneric(_)
                | Unpack(_) | UnpackGeneric(_) | UnpackVariant(_) | UnpackVariantGeneric(_)
                | TestVariant(_) | TestVariantGeneric(_) | VecPack(_, _) | VecUnpack(_, _)
                | PackClosure(_, _) | PackClosureGeneric(_, _) | CallClosure(_) => {
                    mimic_stack.clear();
                }
                Nop => {}
             }
        }

        if cmp {
            println!("Found one trace of cmp {:?} from function {:?}\n", mimic_stack, self.fun_env.get_name_str());
        }
        // The full pattern was not found.
        None
    }
}
