// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]

//! # Grammar
//! ## Identifiers
//! ```text
//! f in FieldName     // [a-zA-Z$_][a-zA-Z0-9$_]*
//! p in ProcedureName // [a-zA-Z$_][a-zA-Z0-9$_]*
//! m in ModuleName    // [a-zA-Z$_][a-zA-Z0-9$_]*
//! n in StructName    // [a-zA-Z$_][a-zA-Z0-9$_]*
//! x in Var           // [a-zA-Z$_][a-zA-Z0-9$_]*
//! ```
//!
//! ## Types
//! ```text
//! k in Kind ::=
//!   | R // Linear resource struct value. Must be used, cannot be copied
//!   | V // Non-resource struct value. Can be silently discarded, can be copied
//!
//! g in GroundType ::=
//!   | bool
//!   | u8        // unsigned 8 bit integer
//!   | u16       // unsigned 16 bit integer
//!   | u32       // unsigned 32 bit integer
//!   | u64       // unsigned 64 bit integer
//!   | u128      // unsigned 128 bit integer
//!   | u256      // unsigned 256 bit integer
//!   | address   // 32 byte account address
//!   | bytearray // immutable, arbitrarily sized array of bytes
//!
//! d in ModuleAlias ::=
//!   | m         // module name that is an alias to a declared module, addr.m
//!   | Self      // current module
//!
//! t in BaseType ::=
//!   | g     // ground type
//!   | k#d.n // struct 'n' declared in the module referenced by 'd' with kind 'k'
//!           // the kind 'k' cannot differ from the declared kind
//!
//! tau in Type ::=
//!   | t      // base type
//!   | &t     // immutable reference to a base type
//!   | &mut t // mutable reference to a base type
//!
//! -list  [Type] ::=
//!   | unit                // empty type list.
//!                         // in the actual syntax, it is represented by the abscense of a type
//!   | tau_1 * ... * tau_j // 'j' >= 1. list of multiple types. used for multiple return values
//! ```
//!
//! ## Values
//! ```text
//! u in Unsigned64        // Unsigned, 64-bit Integer
//! addr in AccountAddress // addresses of blockchain accounts
//! bytes  vector<u8>      // byte array of arbitrary length
//! v in Value ::=
//!   | true
//!   | false
//!   | u        // u64 literal
//!   | 0xaddr   // 32 byte address literal
//!   | b"bytes" // arbitrary length bytearray literal
//! ```
//!
//! ## Expressions
//! ```text
//! o in VarOp ::=
//!   | copy(x) // returns value bound to 'x'
//!   | move(x) // moves the value out of 'x', i.e. returns the value and makes 'x' unusable
//!
//! r in ReferenceOp ::=
//!   | &x        // type: 't -> &mut t'
//!               // creates an exclusive, mutable reference to a local
//!   | &e.f      // type: '&t_1 -> &t_2' or '&mut t_1 -> &mut t_2'
//!               // borrows a new reference to field 'f' of the struct 't_1'. inherits exclusive or shared from parent
//!               // 't_1' must be a struct declared in the current module, i.e. 'f' is "private"
//!   | *e        // type: '&t -> t' or '&mut t -> t'. Dereferencing. Not valid for resources
//!
//! e in Exp ::=
//!   | v
//!   | o
//!   | r
//!   | n { f_1: e_1, ... , f_j: e_j } // type: 'tau-list -> k#Self.n'
//!                                    // "constructor" for 'n'
//!                                    // "packs" the values, binding them to the fields, and creates a new instance of 'n'
//!                                    // 'n' must be declared in the current module
//!   // boolean operators
//!   | !e_1
//!   | e_1 || e_2
//!   | e_1 && e_2
//!   // u64 operators
//!   | e_1 >= e_2
//!   | e_1 <= e_2
//!   | e_1 > e_2
//!   | e_1 < e_2
//!   | e_1 + e_2
//!   | e_1 - e_2
//!   | e_1 * e_2
//!   | e_1 / e_2
//!   | e_1 % e_2
//!   | e_1 ^ e_2
//!   | e_1 | e_2
//!   | e_g & e_2
//!   // operators over any ground type
//!   | e_1 == e_2
//!   | e_1 != e_2
//! ```
//! ## Commands
//! ```text
//! // module operators are available only inside the module that declares n.
//! mop in ModuleOp ::=
//!   | move_from<n>(e)      // type: 'address -> Self.n'
//!                          // removes the resource struct 'n' at the specified address
//!                          // fails if there is no resource present for 'Self.n'
//!   | borrow_global<n>(e)  // type: 'address -> &mut Self.n'
//!                          // borrows a mutable reference to the resource struct 'n' at the specified address
//!                          // fails if there is no resource
//!                          // fails if it is already borrowed in this transaction's execution
//!   | exists<n>(e)         // type: 'address -> bool', s.t. 'n' is a resource struct
//!                          // returns 'true' if the resource struct 'n' at the specified address exists
//!                          // returns 'false' otherwise
//!
//! builtin in Builtin ::=
//!   | create_account(e)         // type: 'addr -> unit'
//!                               // creates new account at the specified address, failing if it already exists
//!   | release(e)                // type: '&t -> unit' or '&mut t -> unit'
//!                               // releases the reference given
//!   | freeze(x)                 // type: '&mut t -> &t'
//!                               // coerce a mutable reference to an immutable reference
//!
//! call in Call ::=
//!   | mop
//!   | builtin
//!   | d.p(e_1, ..., e_j) // procedure 'p' defined in the module referenced by 'd'
//!
//! c in Cmd ::=
//!   | x = e                               // assign the result of evaluating 'e' to 'x'
//!   | x_1, ..., x_j = call                // Invokes 'call', assigns result to 'x_1' to 'x_j'
//!   | call                                // Invokes 'call' that has a return type of 'unit'
//!   | *x = e                              // mutation, s.t. 'x: &mut t' and 'e: t' and 't' is not of resource kind
//!   | assert(e_1, e_2)                    // type: 'bool * u64 -> unit'
//!                                         // halts execution with error code 'e_2' if 'e_1' evaluates to 'false'
//!   | break                               // exit a loop
//!   | continue                            // return to the top of a loop
//!   | return e_1, ..., e_n                // return values from procedure
//!   | n { f_1: x_1, ... , f_j: x_j } = e  // "de-constructor" for 'n'
//!                                         // "unpacks" a struct value 'e: _#Self.n'
//!                                         // value for 'f_i' is bound to local 'x_i'
//! ```
//!
//! ## Statements
//! ```text
//! s in Stmt ::=
//!   | if (e) { s_1 } else { s_2 } // conditional
//!   | if (e) { s }                // conditional without else branch
//!   | while (e) { s }             // while loop
//!   | loop { s }                  // loops forever
//!   | c;                          // command
//!   | s_1 s_2                     // sequencing
//! ```
//!
//! ## Imports
//!```text
//! idecl in Import ::=
//!   | import addr.m_1 as m_2; // imports 'addr.m_1' with the alias 'm_2'
//!   | import addr.m_1;        // imports 'addr.m_1' with the alias 'm_1'
//! ```
//! ## Modules
//! ```text
//! sdecl in StructDecl ::=
//!   | resource n { f_1: t_1, ..., f_j: t_j } // declaration of a resource struct
//!   | struct n { f_1: t_1, ..., f_j: t_j }   // declaration of a non-resource (value) struct
//!                                            // s.t. any 't_i' is not of resource kind
//!
//! body in ProcedureBody ::=
//!  | let x_1; ... let x_j; s // The locals declared in this procedure, and the code for that procedure
//!
//! pdecl in ProcedureDecl ::=
//!   | (public?) p(x_1: tau_1, ..., x_j: tau_j): tau-list { body } // declaration of a defined procedure
//!                                                          // the procedure may be public, or internal to the module
//!   | native (public?) p(x_1: tau_1, ..., x_j: tau_j): tau-list; // declaration of a native procedure
//!                                                         // the implementation is provided by the VM
//!                                                         // the procedure may be public, or internal to the module
//!
//! mdecl in ModuleDecl ::=
//!   | module m { idecl_1 ... idecl_i sdecl_1 ... sdecl_j pdecl_1 ... pdecl_k }
//! ```
//!
//! ## Transaction Scripts
//! ```text
//! TransactionScript ::=
//!   // declaration of the transaction scripts procedure
//!   // the 'main' procedure must be 'public' and any parameters must have a ground type
//!   | idecl_1 ... idecl_i public main(x_1: g_1, ..., x_j: g_j) { s }
//! ```

mod lexer;
pub mod syntax;
