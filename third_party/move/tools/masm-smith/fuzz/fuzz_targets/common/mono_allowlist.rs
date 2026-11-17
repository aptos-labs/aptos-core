// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! MonoMove's known differences from V1, which `mono_oracle` does not report.
//!
//! Each entry matches one exact signature and names a witness test that the Move VM corpus lists
//! in `mono_move_divergences` under the same category. When MonoMove fixes the difference and the
//! corpus drops the witness, the `mono_allowlist` test fails until the entry is removed.

use move_transactional_test_matrix::DivergenceCategory;

/// A divergence accepted at an exact signature.
pub struct AllowedDivergence {
    /// `arm|task kind|v1 status|v2 status|first differing field`, as `mono_oracle` renders it.
    /// In an [`DivergenceCategory::Unsupported`] entry, `*` matches any task kind, V1 status or
    /// field: v2 rejects the construct before running it, whatever V1 then does.
    pub signature: &'static str,
    pub category: DivergenceCategory,
    /// Path of the witness source, relative to the Move VM corpus root.
    pub witness: &'static str,
}

pub const ALLOWED_DIVERGENCES: &[AllowedDivergence] = &[
    AllowedDivergence {
        signature:
            "cross-vm|*|*|UNKNOWN_RUNTIME_STATUS(equality is not supported for this operand \
                    type)|*",
        category: DivergenceCategory::Unsupported,
        witness: "tests/instructions/closure_equality.masm",
    },
    AllowedDivergence {
        signature: "cross-vm|script|NUMBER_OF_TYPE_ARGUMENTS_MISMATCH|ok|outcome",
        category: DivergenceCategory::Semantic,
        witness: "tests/entry_points/script_too_few_type_args_inner.masm",
    },
    AllowedDivergence {
        signature: "cross-vm|script|CONSTRAINT_NOT_SATISFIED|ok|outcome",
        category: DivergenceCategory::Semantic,
        witness: "tests/entry_points/script_type_arg_kind_mismatch_1.masm",
    },
    AllowedDivergence {
        signature: "cross-vm|call|VM_MAX_TYPE_DEPTH_REACHED|ok|outcome",
        category: DivergenceCategory::Semantic,
        witness: "tests/limits/local_type_too_large.masm",
    },
    AllowedDivergence {
        signature: "cross-vm|script|VM_MAX_TYPE_DEPTH_REACHED|ok|outcome",
        category: DivergenceCategory::Semantic,
        witness: "tests/recursion/runtime_type_deeply_nested.masm",
    },
    AllowedDivergence {
        signature: "cross-vm|script|VM_MAX_VALUE_DEPTH_REACHED|ok|outcome",
        category: DivergenceCategory::Semantic,
        witness: "tests/recursion/runtime_layout_deeply_nested.masm",
    },
    AllowedDivergence {
        signature: "cross-vm|call|MISSING_DATA|RUNTIME_DISPATCH_ERROR|major_status",
        category: DivergenceCategory::Semantic,
        witness: "tests/re_entrancy/storage_errors_under_reentry.masm",
    },
    AllowedDivergence {
        signature: "cross-vm|script|MISSING_DATA|RUNTIME_DISPATCH_ERROR|major_status",
        category: DivergenceCategory::Semantic,
        witness: "tests/re_entrancy/storage_errors_under_reentry.masm",
    },
];

impl AllowedDivergence {
    /// Whether `signature` is this divergence.
    pub fn matches(&self, signature: &str) -> bool {
        let wildcards = self.category == DivergenceCategory::Unsupported;
        let fields = signature.split('|');
        self.signature.split('|').count() == fields.clone().count()
            && self
                .signature
                .split('|')
                .zip(fields)
                .all(|(allowed, actual)| allowed == actual || (wildcards && allowed == "*"))
    }
}
