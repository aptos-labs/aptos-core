// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Common helpers shared across MCP tool implementations.

use aptos_types::account_address::AccountAddress;
use move_model::model::{FunctionEnv, GlobalEnv};
use rmcp::model::{CallToolResult, Content};
use std::fmt::Write;

/// Format an `anyhow::Error` with its full cause chain.
pub(crate) fn format_error_chain(err: &anyhow::Error) -> String {
    let mut msg = String::new();
    for (i, cause) in err.chain().enumerate() {
        if i > 0 {
            write!(msg, ": ").unwrap();
        }
        write!(msg, "{}", cause).unwrap();
    }
    msg
}

/// Convert an `rmcp::ErrorData` into a `CallToolResult::error` so the AI
/// client always sees the message. Used as the `unwrap_or_else` handler in
/// `#[tool]` methods that delegate to an `_impl` method.
pub(crate) fn tool_error(e: rmcp::ErrorData) -> CallToolResult {
    log::error!("{}", e.message);
    CallToolResult::error(vec![Content::text(e.message.to_string())])
}

/// Shorthand for creating an `rmcp::ErrorData` internal error.
pub(crate) fn mcp_err(msg: impl Into<String>) -> rmcp::ErrorData {
    rmcp::ErrorData::internal_error(msg.into(), None)
}

/// Like `mcp_err` but formats an `anyhow::Error` cause chain after a prefix.
pub(crate) fn mcp_err_chain(prefix: &str, e: &anyhow::Error) -> rmcp::ErrorData {
    mcp_err(format!("{}: {}", prefix, format_error_chain(e)))
}

/// Call `f`, catching any panic and converting it to an MCP error with `msg`.
pub(crate) fn try_call<T>(msg: &str, f: impl FnOnce() -> T) -> Result<T, rmcp::ErrorData> {
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(f)).map_err(|_| mcp_err(msg))
}

/// Shorthand for creating an `rmcp::ErrorData` invalid params error.
pub(crate) fn mcp_invalid(msg: impl Into<String>) -> rmcp::ErrorData {
    rmcp::ErrorData::invalid_params(msg.into(), None)
}

/// Resolve a string to a FunctionEnv.
/// Accepts "module::function" or "address::module::function".
pub(crate) fn resolve_function<'a>(
    env: &'a GlobalEnv,
    function: &str,
) -> Result<FunctionEnv<'a>, rmcp::ErrorData> {
    let parts: Vec<&str> = function.split("::").collect();
    let (addr_filter, module_name, func_name) = match parts.as_slice() {
        [module, func] => (None::<AccountAddress>, *module, *func),
        [addr, module, func] => (Some(resolve_address(env, addr)?), *module, *func),
        _ => return Err(mcp_invalid(format!(
            "invalid function format `{}`, expected `module::function` or `address::module::function`",
            function
        ))),
    };

    let module_sym = env.symbol_pool().make(module_name);
    let module = env
        .get_primary_target_modules()
        .into_iter()
        .find(|m| {
            m.get_name().name() == module_sym
                && addr_filter.is_none_or(|a| m.self_address().expect_numerical() == a)
        })
        .ok_or_else(|| {
            mcp_invalid(format!(
                "no module `{}` found in the target package",
                module_name
            ))
        })?;
    let func_sym = env.symbol_pool().make(func_name);
    module.find_function(func_sym).ok_or_else(|| {
        mcp_invalid(format!(
            "no function `{}` found in module `{}`",
            func_name, module_name
        ))
    })
}

/// Parse an address string into an AccountAddress.
/// Supports hex literals ("0xCAFE") and named aliases.
fn resolve_address(env: &GlobalEnv, addr: &str) -> Result<AccountAddress, rmcp::ErrorData> {
    if let Ok(a) = AccountAddress::from_hex_literal(addr) {
        return Ok(a);
    }
    let sym = env.symbol_pool().make(addr);
    env.resolve_address_alias(sym)
        .ok_or_else(|| mcp_invalid(format!("cannot resolve address `{}`", addr)))
}
