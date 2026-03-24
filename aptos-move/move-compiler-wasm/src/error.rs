// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License

//! Error types for Move compiler WASM

use std::fmt;

/// Errors that can occur during compilation in WASM
#[derive(Debug, Clone)]
pub enum CompilerError {
    /// Invalid address format
    InvalidAddress(String),

    /// Compilation failed with errors
    CompilationFailed(Vec<String>),

    /// No bytecode was generated
    NoBytecodeGenerated,

    /// Package configuration error
    PackageError(String),

    /// Internal error
    InternalError(String),
}

impl fmt::Display for CompilerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CompilerError::InvalidAddress(msg) => write!(f, "Invalid address: {}", msg),
            CompilerError::CompilationFailed(errors) => {
                write!(f, "Compilation failed with {} error(s):\n", errors.len())?;
                for err in errors {
                    write!(f, "  - {}\n", err)?;
                }
                Ok(())
            }
            CompilerError::NoBytecodeGenerated => write!(f, "No bytecode was generated"),
            CompilerError::PackageError(msg) => write!(f, "Package error: {}", msg),
            CompilerError::InternalError(msg) => write!(f, "Internal error: {}", msg),
        }
    }
}

impl std::error::Error for CompilerError {}
