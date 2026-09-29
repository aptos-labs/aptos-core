// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Rust mirror of the RawUnit JSON v1 subset emitted by the M0 mapper.
//!
//! The types deliberately spell every serialized field instead of passing a
//! Rustc Public serde representation through to Lean. Empty arrays use an
//! uninhabited element type until the corresponding mapper slice lands, so an
//! unsupported declaration cannot accidentally enter the exchange.

use serde::{Deserialize, Serialize};
use serde_json::Number;

macro_rules! id_type {
    ($($name:ident),+ $(,)?) => {$ (
        #[derive(Clone, Copy, Debug, Deserialize, Serialize)]
        #[serde(deny_unknown_fields)]
        pub(super) struct $name {
            pub index: usize,
        }

        impl $name {
            #[allow(dead_code)]
            pub(super) const fn new(index: usize) -> Self {
                Self { index }
            }
        }
    )+ };
}

id_type!(
    AlignmentId,
    EvidenceId,
    ExprId,
    FileId,
    LocId,
    LocalId,
    LifetimeId,
    NameId,
    NamespaceId,
    OriginId,
    PlaceId,
    TypeId,
);

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub(super) struct QualifiedRef {
    pub name: NameId,
    pub namespace_id: NamespaceId,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(super) enum UnsupportedNode {}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub(super) struct ImportEvidence {
    pub description: String,
    pub producer: String,
    pub trusted: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub(super) enum Profile {
    Rust,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub(super) struct Version {
    pub major: usize,
    pub minor: usize,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub(super) struct SourceFile {
    pub content_hash: String,
    pub name: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub(super) struct SourceRange {
    pub end_byte: usize,
    pub file: FileId,
    pub start_byte: usize,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub(super) struct Location {
    pub expansion: Vec<SourceRange>,
    pub generated_by: Option<String>,
    pub parent: Option<LocId>,
    pub primary: Option<SourceRange>,
    pub related: Vec<SourceRange>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub(super) struct Comment {
    pub is_doc: bool,
    pub loc: LocId,
    pub own_line: bool,
    pub text: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub(super) enum OriginKind {
    RustMir,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub(super) struct Origin {
    pub description: String,
    pub kind: OriginKind,
    pub location: LocId,
    pub source_identity: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub(super) enum Trust {
    Checked,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub(super) struct Alignment {
    pub description: String,
    pub source: OriginId,
    pub trust: Trust,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub(super) struct NamespaceRef {
    pub segments: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub(super) struct QualifiedName {
    pub name: String,
    pub namespace_id: NamespaceId,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub(super) enum Ty {
    Unit,
    Never,
    Bool,
    Character,
    String,
    Integer {
        signed: bool,
        width: IntWidth,
    },
    Tuple {
        elements: Vec<TypeId>,
    },
    Vector {
        element: TypeId,
        length: Option<ConstValue>,
    },
    Nominal {
        arguments: Vec<GenericArgument>,
        name: NameId,
    },
    TypeParameter {
        index: usize,
    },
    Reference {
        value: ReferenceType,
    },
    Function {
        abilities: Vec<Ability>,
        arguments: Vec<TypeId>,
        result: TypeId,
    },
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub(super) enum Ability {
    Copy,
    Drop,
    Store,
    Key,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(
    deny_unknown_fields,
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub(super) enum GenericArgument {
    TypeArg { value: TypeUse },
    Const { value: ConstValue },
    Lifetime { value: LifetimeId },
    Evidence { value: EvidenceId },
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub(super) enum BinderKind {
    TypeArg,
    Const,
    Lifetime,
    Evidence,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub(super) struct GenericBinder {
    pub abilities: Vec<UnsupportedNode>,
    pub kind: BinderKind,
    pub loc: LocId,
    pub name: String,
    pub predicates: Vec<UnsupportedNode>,
    #[serde(rename = "type")]
    pub type_: Option<TypeUse>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub(super) struct ReferenceType {
    pub kind: ReferenceKind,
    pub lifetime: LifetimeId,
    pub profile: Profile,
    pub referent: TypeId,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub(super) enum ReferenceKind {
    Shared,
    Mutable,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub(super) struct Lifetime {
    pub kind: LifetimeKind,
    pub loc: LocId,
    pub name: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(
    deny_unknown_fields,
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub(super) enum LifetimeKind {
    Static,
    Parameter { index: usize },
    Inference,
    Local,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(
    deny_unknown_fields,
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub(super) enum IntWidth {
    Bits { width: usize },
    Pointer,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub(super) struct Tables {
    pub alignments: Vec<Alignment>,
    pub files: Vec<SourceFile>,
    pub lifetimes: Vec<Lifetime>,
    pub locations: Vec<Location>,
    pub names: Vec<QualifiedName>,
    pub namespaces: Vec<NamespaceRef>,
    pub origins: Vec<Origin>,
    pub types: Vec<Ty>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub(super) struct ProfileConfig {
    pub name: String,
    pub options: Vec<(String, String)>,
    pub profile: Profile,
    pub version: usize,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub(super) struct TypeUse {
    pub loc: LocId,
    pub type_id: TypeId,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(
    deny_unknown_fields,
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub(super) enum ConstValue {
    Unit,
    Bool { value: bool },
    Character { value: usize },
    Integer { value: Number },
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(
    deny_unknown_fields,
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub(super) enum ExprKind {
    Value {
        source_constant: Option<String>,
        value: ConstValue,
    },
    Assign {
        place: PlaceId,
        value: ExprId,
    },
    LocalVar {
        local_id: LocalId,
    },
    Operation {
        arguments: Vec<ExprId>,
        instantiations: Vec<GenericArgument>,
        operation: Operation,
        surface: Option<UnsupportedNode>,
    },
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(
    deny_unknown_fields,
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub(super) enum Operation {
    Move { place: PlaceId },
    Copy { place: PlaceId },
    Read { place: PlaceId },
    Borrow { kind: BorrowKind, place: PlaceId },
    Reference { kind: ReferenceOperation },
    Primitive { kind: PrimitiveOperation },
    Data { kind: DataOperation },
    Call { kind: CallKind },
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub(super) enum ReferenceOperation {
    Dereference,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(
    deny_unknown_fields,
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub(super) enum DataOperation {
    Discriminant {
        #[serde(rename = "type")]
        type_: QualifiedRef,
    },
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub(super) enum BorrowKind {
    Immutable,
    Mutable,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(
    deny_unknown_fields,
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub(super) enum CallKind {
    Function {
        callee: QualifiedRef,
    },
    Constructor {
        constructor: QualifiedRef,
        variant: Option<String>,
    },
    Closure {
        function: QualifiedRef,
    },
    Invoke,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub(super) enum PrimitiveOperation {
    Tuple,
    Vector,
    RepeatVector,
    Length,
    Add,
    OverflowingAdd,
    Subtract,
    OverflowingSubtract,
    Multiply,
    OverflowingMultiply,
    Divide,
    Modulo,
    BitwiseAnd,
    BitwiseOr,
    BitwiseXor,
    BitwiseNot,
    ShiftLeft,
    ShiftRight,
    Equal,
    NotEqual,
    Less,
    LessEqual,
    Greater,
    GreaterEqual,
    LogicalNot,
    Negate,
    Cast,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub(super) struct Expr {
    pub kind: ExprKind,
    pub loc: LocId,
    pub type_id: TypeId,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(
    deny_unknown_fields,
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub(super) enum Place {
    LocalVar {
        local_id: LocalId,
    },
    Deref {
        base: PlaceId,
    },
    Field {
        base: PlaceId,
        owner: QualifiedRef,
        field: NameId,
    },
    Index {
        base: PlaceId,
        index: ExprId,
    },
    Subslice {
        base: PlaceId,
        from_end: bool,
        start: usize,
        stop: usize,
    },
    Downcast {
        base: PlaceId,
        variant: NameId,
    },
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(
    deny_unknown_fields,
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub(super) enum RawStatement {
    Execute { expression: ExprId },
    StorageLive { local_id: LocalId },
    StorageDead { local_id: LocalId },
    Deinit { place: PlaceId },
    SetDiscriminant { place: PlaceId, variant: NameId },
    Retag { place: PlaceId },
    PlaceMention { place: PlaceId },
    AscribeUserType { place: PlaceId, r#type: TypeUse },
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(
    deny_unknown_fields,
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub(super) enum RawAssertKind {
    BoundsCheck,
    Overflow,
    DivisionByZero,
    RemainderByZero,
    MisalignedPointerDereference,
    Profile { value: UnsupportedNode },
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(
    deny_unknown_fields,
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub(super) enum RawTerminator {
    Goto {
        target: BlockId,
    },
    Branch {
        condition: ExprId,
        else_target: BlockId,
        then_target: BlockId,
    },
    Switch {
        cases: Vec<(ConstValue, BlockId)>,
        default_target: BlockId,
        scrutinee: ExprId,
    },
    Call {
        call: ExprId,
        destination: Option<RawCallDestination>,
        unwind: RawUnwindAction,
    },
    Drop {
        place: PlaceId,
        target: BlockId,
        unwind: RawUnwindAction,
    },
    Assert {
        condition: ExprId,
        expected: bool,
        kind: RawAssertKind,
        target: BlockId,
        unwind: RawUnwindAction,
    },
    #[serde(rename = "return_")]
    Return {
        values: Vec<ExprId>,
    },
    Unreachable,
    Resume,
    Abort,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub(super) struct RawCallDestination {
    pub place: PlaceId,
    pub target: BlockId,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(
    deny_unknown_fields,
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub(super) enum RawUnwindAction {
    #[serde(rename = "continue_")]
    Continue,
    Unreachable,
    Terminate {
        reason: String,
    },
    Cleanup {
        target: BlockId,
    },
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub(super) struct RawBasicBlock {
    pub loc: LocId,
    pub statements: Vec<RawStatement>,
    pub terminator: RawTerminator,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub(super) struct RawCfg {
    pub blocks: Vec<RawBasicBlock>,
    pub entry: BlockId,
}

id_type!(BlockId);

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(
    deny_unknown_fields,
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub(super) enum RawBody {
    Cfg { graph: RawCfg },
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub(super) struct Parameter {
    pub mutable: bool,
    pub name: String,
    pub type_use: TypeUse,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub(super) struct Signature {
    pub generics: Vec<GenericBinder>,
    pub parameters: Vec<Parameter>,
    pub predicates: Vec<UnsupportedNode>,
    pub results: Vec<TypeUse>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub(super) struct LocalDecl {
    pub id: LocalId,
    pub loc: LocId,
    pub mutable: bool,
    pub name: String,
    #[serde(rename = "type")]
    pub type_use: TypeUse,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub(super) struct FunctionContract {
    pub conditions: Vec<UnsupportedNode>,
    pub has_frame: bool,
    pub loc: Option<LocId>,
    pub modifies: Vec<ExprId>,
    pub modifies_all: bool,
    pub pragmas: Vec<UnsupportedNode>,
    pub reads: Vec<TypeUse>,
    pub reads_all: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub(super) struct FunctionDecl {
    pub alignment: AlignmentId,
    pub attributes: Vec<UnsupportedNode>,
    pub body: RawBody,
    pub contract: FunctionContract,
    pub doc: String,
    pub loc: LocId,
    pub locals: Vec<LocalDecl>,
    pub name: NameId,
    pub origin: OriginId,
    pub pragmas: Vec<UnsupportedNode>,
    pub profile: Profile,
    pub profile_data: Vec<UnsupportedNode>,
    pub signature: Signature,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub(super) struct FieldDecl {
    pub doc: String,
    pub loc: LocId,
    pub name: NameId,
    #[serde(rename = "type")]
    pub type_use: TypeUse,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub(super) struct VariantDecl {
    pub discriminant: Option<Number>,
    pub fields: Vec<FieldDecl>,
    pub loc: LocId,
    pub name: NameId,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub(super) struct StructDecl {
    pub abilities: Vec<UnsupportedNode>,
    pub attributes: Vec<UnsupportedNode>,
    pub contract: FunctionContract,
    pub doc: String,
    pub fields: Vec<FieldDecl>,
    pub generics: Vec<GenericBinder>,
    pub loc: LocId,
    pub locals: Vec<UnsupportedNode>,
    pub name: NameId,
    pub properties: Vec<UnsupportedNode>,
    pub variants: Vec<VariantDecl>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub(super) struct RawNamespace {
    pub associated_items: Vec<UnsupportedNode>,
    pub attributes: Vec<UnsupportedNode>,
    pub comments: Vec<Comment>,
    pub constants: Vec<UnsupportedNode>,
    pub doc: String,
    pub expressions: Vec<Expr>,
    pub functions: Vec<FunctionDecl>,
    pub identity: NamespaceId,
    pub implementations: Vec<UnsupportedNode>,
    pub imports: Vec<NamespaceId>,
    pub intrinsics: Vec<UnsupportedNode>,
    pub invariants: Vec<UnsupportedNode>,
    pub loc: LocId,
    pub patterns: Vec<UnsupportedNode>,
    pub places: Vec<Place>,
    pub pragmas: Vec<UnsupportedNode>,
    pub profile: Option<Profile>,
    pub profile_metadata: Vec<UnsupportedNode>,
    pub spec_functions: Vec<UnsupportedNode>,
    pub spec_vars: Vec<UnsupportedNode>,
    pub structs: Vec<StructDecl>,
    pub traits: Vec<UnsupportedNode>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub(super) struct RawUnit {
    pub dependencies: Vec<UnsupportedNode>,
    pub evidence: Vec<ImportEvidence>,
    pub namespaces: Vec<RawNamespace>,
    pub profiles: Vec<ProfileConfig>,
    pub tables: Tables,
    pub version: Version,
}

/// Add deterministic review-friendly whitespace without reparsing JSON numbers.
/// `serde_json`'s generic pretty path rounds its arbitrary-precision `Number`
/// carrier through a floating representation, so formatting must operate on
/// the already exact compact spelling.
fn pretty_exact_json(compact: &str) -> String {
    let characters: Vec<char> = compact.chars().collect();
    let mut output = String::with_capacity(compact.len() + compact.len() / 2);
    let mut indent = 0usize;
    let mut in_string = false;
    let mut escaped = false;
    let mut index = 0usize;
    let newline = |output: &mut String, indent: usize| {
        output.push('\n');
        output.extend(std::iter::repeat_n(' ', indent * 2));
    };
    while index < characters.len() {
        let character = characters[index];
        if in_string {
            output.push(character);
            if escaped {
                escaped = false;
            } else if character == '\\' {
                escaped = true;
            } else if character == '"' {
                in_string = false;
            }
            index += 1;
            continue;
        }
        match character {
            '"' => {
                in_string = true;
                output.push(character);
            },
            '{' | '[' => {
                let closing = if character == '{' { '}' } else { ']' };
                output.push(character);
                if characters.get(index + 1) == Some(&closing) {
                    output.push(closing);
                    index += 1;
                } else {
                    indent += 1;
                    newline(&mut output, indent);
                }
            },
            '}' | ']' => {
                indent = indent.saturating_sub(1);
                newline(&mut output, indent);
                output.push(character);
            },
            ',' => {
                output.push(character);
                newline(&mut output, indent);
            },
            ':' => output.push_str(": "),
            _ => output.push(character),
        }
        index += 1;
    }
    output.push('\n');
    output
}

pub(super) fn encode(unit: &RawUnit) -> Result<String, String> {
    serde_json::to_string(unit)
        .map(|text| pretty_exact_json(&text))
        .map_err(|error| format!("serialize RawUnit JSON: {error}"))
}

pub(super) fn decode(text: &str) -> Result<RawUnit, String> {
    serde_json::from_str(text).map_err(|error| format!("decode RawUnit JSON mirror: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn comments_match_the_shared_json_schema() {
        let comment = Comment {
            is_doc: true,
            loc: LocId::new(4),
            own_line: true,
            text: "/// docs".to_owned(),
        };
        assert_eq!(
            serde_json::to_string(&comment).unwrap(),
            r#"{"isDoc":true,"loc":{"index":4},"ownLine":true,"text":"/// docs"}"#
        );
    }

    #[test]
    fn raw_mir_administration_and_assertions_match_the_shared_json_schema() {
        let storage = RawStatement::StorageLive {
            local_id: LocalId::new(2),
        };
        assert_eq!(
            serde_json::to_string(&storage).unwrap(),
            r#"{"storageLive":{"localId":{"index":2}}}"#
        );
        let storage_dead = RawStatement::StorageDead {
            local_id: LocalId::new(2),
        };
        assert_eq!(
            serde_json::to_string(&storage_dead).unwrap(),
            r#"{"storageDead":{"localId":{"index":2}}}"#
        );
        let deinit = RawStatement::Deinit {
            place: PlaceId::new(5),
        };
        assert_eq!(
            serde_json::to_string(&deinit).unwrap(),
            r#"{"deinit":{"place":{"index":5}}}"#
        );
        let discriminant = RawStatement::SetDiscriminant {
            place: PlaceId::new(5),
            variant: NameId::new(8),
        };
        assert_eq!(
            serde_json::to_string(&discriminant).unwrap(),
            r#"{"setDiscriminant":{"place":{"index":5},"variant":{"index":8}}}"#
        );
        let retag = RawStatement::Retag {
            place: PlaceId::new(5),
        };
        assert_eq!(
            serde_json::to_string(&retag).unwrap(),
            r#"{"retag":{"place":{"index":5}}}"#
        );
        let mention = RawStatement::PlaceMention {
            place: PlaceId::new(5),
        };
        assert_eq!(
            serde_json::to_string(&mention).unwrap(),
            r#"{"placeMention":{"place":{"index":5}}}"#
        );
        let ascription = RawStatement::AscribeUserType {
            place: PlaceId::new(5),
            r#type: TypeUse {
                loc: LocId::new(6),
                type_id: TypeId::new(7),
            },
        };
        assert_eq!(
            serde_json::to_string(&ascription).unwrap(),
            r#"{"ascribeUserType":{"place":{"index":5},"type":{"loc":{"index":6},"typeId":{"index":7}}}}"#
        );
        let assertion = RawTerminator::Assert {
            condition: ExprId::new(3),
            expected: false,
            kind: RawAssertKind::DivisionByZero,
            target: BlockId::new(4),
            unwind: RawUnwindAction::Unreachable,
        };
        assert_eq!(
            serde_json::to_string(&assertion).unwrap(),
            r#"{"assert":{"condition":{"index":3},"expected":false,"kind":"divisionByZero","target":{"index":4},"unwind":"unreachable"}}"#
        );
    }

    #[test]
    fn full_range_unsigned_integers_are_unquoted_json_numbers() {
        let value = ConstValue::Integer {
            value: Number::from_u128(u128::MAX).unwrap(),
        };
        let json = serde_json::to_string(&value).unwrap();
        assert_eq!(
            json,
            r#"{"integer":{"value":340282366920938463463374607431768211455}}"#
        );
        let _: ConstValue = serde_json::from_str(&json).unwrap();
        let pretty = pretty_exact_json(&json);
        assert!(pretty.contains("340282366920938463463374607431768211455"));
        assert!(!pretty.contains("e+"));
        let decoded: ConstValue = serde_json::from_str(&pretty).unwrap();
        assert_eq!(serde_json::to_string(&decoded).unwrap(), json);
    }

    #[test]
    fn generic_arguments_match_the_shared_json_schema() {
        let arguments = vec![
            GenericArgument::TypeArg {
                value: TypeUse {
                    loc: LocId::new(1),
                    type_id: TypeId::new(2),
                },
            },
            GenericArgument::Const {
                value: ConstValue::Integer {
                    value: Number::from(3),
                },
            },
            GenericArgument::Lifetime {
                value: LifetimeId::new(4),
            },
            GenericArgument::Evidence {
                value: EvidenceId::new(5),
            },
        ];
        let json = serde_json::to_string(&arguments).unwrap();
        let decoded: Vec<GenericArgument> = serde_json::from_str(&json).unwrap();
        assert_eq!(serde_json::to_string(&decoded).unwrap(), json);
    }

    #[test]
    fn mirror_rejects_unknown_record_and_variant_fields() {
        assert!(serde_json::from_str::<Version>(r#"{"major":1,"minor":0,"patch":0}"#).is_err());
        assert!(serde_json::from_str::<ImportEvidence>(
            r#"{"description":"checked","producer":"rustc","trusted":true,"hash":"missing"}"#
        )
        .is_err());
        assert!(
            serde_json::from_str::<ConstValue>(r#"{"integer":{"value":7,"signed":false}}"#)
                .is_err()
        );
    }

    #[test]
    fn mirror_rejects_duplicate_record_and_variant_fields() {
        assert!(serde_json::from_str::<Version>(r#"{"major":1,"major":1,"minor":0}"#).is_err());
        assert!(serde_json::from_str::<ImportEvidence>(
            r#"{"description":"checked","producer":"rustc","producer":"other","trusted":true}"#
        )
        .is_err());
        assert!(
            serde_json::from_str::<ConstValue>(r#"{"integer":{"value":7,"value":8}}"#).is_err()
        );
    }
}
