// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Natives for the `string_utils` module.

use crate::{polymorphic_natives, NativeEntry};
use mono_move_core::{
    native::{NativeContext, NativeContextFamily, NativeStatus, Opaque, Ref, Vector},
    types::{is_nominal, view_type, view_type_list, InternedType, Type},
    FormatOptions, VMResult,
};
use move_core_types::account_address::AccountAddress;

/// A `{}` substitution did not line up with the value list it formats.
const EARGS_MISMATCH: u64 = 1;
/// The format string is not well formed.
const EINVALID_FORMAT: u64 = 2;

/// `0x1::string_utils::native_format<T>(s: &T, type_tag: bool, canonicalize:
/// bool, single_line: bool, include_int_types: bool): String`
//
// TODO(metering): charge gas.
pub fn native_format<C: NativeContext>(ctx: &C) -> VMResult<NativeStatus> {
    let ty = ctx.ty_arg(0)?;
    // SAFETY: arg 0 is the reference `&T`, whose pointee type is `ty`.
    let arg: Ref<Opaque> = unsafe { ctx.arg(0)? };

    let mut options = FormatOptions::MONO_MOVE;
    // SAFETY: args 1..5 are the four syntax flags, in declaration order.
    unsafe {
        options.fully_qualified_nominals = ctx.arg::<bool>(1)?;
        options.canonical_addresses = ctx.arg::<bool>(2)?;
        options.single_line = ctx.arg::<bool>(3)?;
        options.int_suffixes = ctx.arg::<bool>(4)?;
    }

    // SAFETY: `arg` references a live value of type `ty` for the rest of the call.
    let formatted = unsafe { ctx.format_value(arg.ptr(), ty, &options)? };
    let out = ctx.new_byte_vector(formatted.as_bytes())?;
    // SAFETY: return 0 is `String`, which has the same representation as
    // `vector<u8>`.
    unsafe { ctx.set_return(0, out)? };
    Ok(NativeStatus::Success)
}

/// `0x1::string_utils::native_format_list<T>(fmt: &vector<u8>, val: &T): String`
///
/// Substitutes each `{}` in `fmt` with the next element of the cons list `val`,
/// leaving `{{` and `}}` as literal braces.
//
// TODO(metering): charge gas.
pub fn native_format_list<C: NativeContext>(ctx: &C) -> VMResult<NativeStatus> {
    // SAFETY: arg 0 is `&vector<u8>`.
    let fmt_arg: Ref<Vector<u8>> = unsafe { ctx.arg(0)? };
    // SAFETY: arg 1 is the reference `&T`, whose pointee type is `ty_arg(0)`.
    let list: Ref<Opaque> = unsafe { ctx.arg(1)? };

    let fmt_vec = fmt_arg.borrow();
    // SAFETY: the bytes are consumed before any allocation, so GC cannot
    // relocate them while the slice is held.
    let Ok(fmt) = std::str::from_utf8(unsafe { fmt_vec.as_bytes() }) else {
        return Ok(abort(EINVALID_FORMAT, None));
    };

    let mut cursor = Cursor {
        ptr: list.ptr(),
        ty: ctx.ty_arg(0)?,
    };
    let mut out = String::new();
    // `1` after an opening `{`, `-1` after a closing `}`, `0` otherwise. The
    // next character decides whether the brace was a substitution, an escape,
    // or a malformed string.
    let mut in_braces = 0i8;
    for c in fmt.chars() {
        if in_braces == 1 {
            in_braces = 0;
            if c == '}' {
                match cursor.pop(ctx, &mut out)? {
                    Ok(next) => cursor = next,
                    Err(status) => return Ok(status),
                }
                continue;
            } else if c != '{' {
                return Ok(abort(
                    EINVALID_FORMAT,
                    Some("Invalid format string: unmatched '{{' bracket"),
                ));
            }
        } else if in_braces == -1 {
            in_braces = 0;
            if c != '}' {
                return Ok(abort(
                    EINVALID_FORMAT,
                    Some("Invalid format string: unmatched '}}' bracket"),
                ));
            }
        } else if c == '{' {
            in_braces = 1;
            continue;
        } else if c == '}' {
            in_braces = -1;
            continue;
        }
        out.push(c);
    }
    if in_braces != 0 {
        return Ok(abort(
            EINVALID_FORMAT,
            Some("Invalid format string: unclosed brackets"),
        ));
    }
    if !is_nominal(cursor.ty, &AccountAddress::ONE, "string_utils", "NIL") {
        return Ok(abort(EARGS_MISMATCH, None));
    }

    let bytes = ctx.new_byte_vector(out.as_bytes())?;
    // SAFETY: return 0 is `String`, which has the same representation as
    // `vector<u8>`.
    unsafe { ctx.set_return(0, bytes)? };
    Ok(NativeStatus::Success)
}

/// Position in the cons list `Cons<T, N> { car: T, cdr: N }` being formatted.
#[derive(Clone, Copy)]
struct Cursor {
    ptr: *const u8,
    ty: InternedType,
}

impl Cursor {
    /// Formats `car` into `out` and returns the cursor for `cdr`. The inner
    /// `Err` aborts the native: the list ran out, or was never a list at all.
    fn pop<C: NativeContext>(
        self,
        ctx: &C,
        out: &mut String,
    ) -> VMResult<Result<Self, NativeStatus>> {
        if !is_nominal(self.ty, &AccountAddress::ONE, "string_utils", "Cons") {
            return Ok(Err(abort(EARGS_MISMATCH, None)));
        }
        let Type::Nominal { ty_args, .. } = view_type(self.ty) else {
            return Ok(Err(abort(EARGS_MISMATCH, None)));
        };
        let [car_ty, cdr_ty] = view_type_list(*ty_args) else {
            return Ok(Err(abort(EARGS_MISMATCH, None)));
        };
        let (Some(car_offset), Some(cdr_offset)) =
            (ctx.field_offset(self.ty, 0)?, ctx.field_offset(self.ty, 1)?)
        else {
            return Ok(Err(abort(EARGS_MISMATCH, None)));
        };

        // SAFETY: `self.ptr` references a live `Cons`, so both fields lie at
        // their offsets within it.
        unsafe {
            let car = self.ptr.add(car_offset as usize);
            display(ctx, car, *car_ty, out)?;
            Ok(Ok(Self {
                ptr: self.ptr.add(cdr_offset as usize),
                ty: *cdr_ty,
            }))
        }
    }
}

/// Formats one list element.
///
/// # Safety
///
/// `ptr` must reference a live value of type `ty`.
unsafe fn display<C: NativeContext>(
    ctx: &C,
    ptr: *const u8,
    ty: InternedType,
    out: &mut String,
) -> VMResult<()> {
    // SAFETY: forwarded from this function's contract.
    let formatted = unsafe { ctx.format_value(ptr, ty, &FormatOptions::LIST_ELEMENT)? };
    out.push_str(&formatted);
    Ok(())
}

/// The messages, including their doubled braces, reproduce V1 byte for byte.
/// V1 attaches none to `EARGS_MISMATCH`.
fn abort(code: u64, message: Option<&str>) -> NativeStatus {
    NativeStatus::Abort {
        code,
        message: message.map(str::to_string),
    }
}

/// Natives for the `string_utils` module.
pub fn make_all_string_utils_natives<F: NativeContextFamily>() -> Vec<NativeEntry<F>> {
    polymorphic_natives![
        ("0x1::string_utils::native_format", native_format),
        ("0x1::string_utils::native_format_list", native_format_list),
    ]
}
