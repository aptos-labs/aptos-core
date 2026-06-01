// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Visitor-based traversal of the canonical [`Type`] DAG with lazy type
//! paramter resolution and no new type creation.
//!
//! Specifically, walking a generic type never materializes the substituted
//! type. For example, given `vector<T>` and substitution `T = u64`, the type
//! for `vector<u64>` is never created. Instead, the original generic type is
//! traversed and its type paramters are resolved to concrete types under the
//! current environment on demand.

use mono_move_core::{
    interner::{InternedIdentifier, InternedModuleId},
    types::{view_type, view_type_list, InternedType, InternedTypeList, Type},
    FieldTypes,
};
use move_core_types::ability::AbilitySet;
use smallvec::{smallvec, SmallVec};

/// Maximum nesting depth allowed during traversal.
const MAX_TRAVERSAL_DEPTH: u32 = 128;

/// Error returned during the type walk.
#[derive(Debug, thiserror::Error)]
pub enum VisitError<E> {
    #[error("type nesting depth exceeded")]
    DepthExceeded,
    #[error("unbound type parameter at index {idx}")]
    UnboundTypeParam { idx: u16 },
    #[error("custom error")]
    Custom(E),
}

/// Callbacks invoked when visiting each type node. By default, the visitor
/// resolves type parameters and bounds recursion depth. The implementations
/// only need to decide what each node contributes and implement the descent
/// into composite types.
pub trait TypeVisitor {
    type Error;

    fn visit_bool(&mut self) -> Result<(), Self::Error>;

    fn visit_u8(&mut self) -> Result<(), Self::Error>;

    fn visit_u16(&mut self) -> Result<(), Self::Error>;

    fn visit_u32(&mut self) -> Result<(), Self::Error>;

    fn visit_u64(&mut self) -> Result<(), Self::Error>;

    fn visit_u128(&mut self) -> Result<(), Self::Error>;

    fn visit_u256(&mut self) -> Result<(), Self::Error>;

    fn visit_i8(&mut self) -> Result<(), Self::Error>;

    fn visit_i16(&mut self) -> Result<(), Self::Error>;

    fn visit_i32(&mut self) -> Result<(), Self::Error>;

    fn visit_i64(&mut self) -> Result<(), Self::Error>;

    fn visit_i128(&mut self) -> Result<(), Self::Error>;

    fn visit_i256(&mut self) -> Result<(), Self::Error>;

    fn visit_address(&mut self) -> Result<(), Self::Error>;

    fn visit_signer(&mut self) -> Result<(), Self::Error>;

    fn visit_vector(
        &mut self,
        ctx: &mut VisitorCtx,
        scope: Scope,
        elem: InternedType,
    ) -> Result<(), VisitError<Self::Error>>;

    fn visit_immut_ref(
        &mut self,
        ctx: &mut VisitorCtx,
        scope: Scope,
        inner: InternedType,
    ) -> Result<(), VisitError<Self::Error>>;

    fn visit_mut_ref(
        &mut self,
        ctx: &mut VisitorCtx,
        scope: Scope,
        inner: InternedType,
    ) -> Result<(), VisitError<Self::Error>>;

    fn visit_function(
        &mut self,
        ctx: &mut VisitorCtx,
        scope: Scope,
        args: InternedTypeList,
        results: InternedTypeList,
        abilities: AbilitySet,
    ) -> Result<(), VisitError<Self::Error>>;

    fn visit_nominal(
        &mut self,
        ctx: &mut VisitorCtx,
        scope: Scope,
        module_id: InternedModuleId,
        name: InternedIdentifier,
        ty_args: InternedTypeList,
    ) -> Result<(), VisitError<Self::Error>>;
}

/// Inline capacity for per-frame resolved type argument caches.
const INLINE_TY_ARGS: usize = 4;

/// A single environment scope: a nominal's type arguments, the frame in which
/// those arguments' free type parameters are bound (`parent`), and an inline
/// cache of resolved arguments. Each cache entry stores the resolved type
/// together with the frame its own free type parameters are bound in.
struct EnvFrame {
    ty_args: InternedTypeList,
    parent: usize,
    cache: SmallVec<[Option<(InternedType, usize)>; INLINE_TY_ARGS]>,
}

impl EnvFrame {
    /// Returns a new environment for type arguments bound in `parent`, with an
    /// empty cache.
    fn new(ty_args: InternedTypeList, parent: usize) -> Self {
        let n = view_type_list(ty_args).len();
        Self {
            ty_args,
            parent,
            cache: smallvec![None; n],
        }
    }
}

/// Per-node cursor threaded down the walk: the frame the node's type parameters
/// resolve against, and the current nesting depth (used to bound recursion).
/// Opaque to handlers — they receive it and pass it back to [`visit`].
#[derive(Clone, Copy)]
pub struct Scope {
    frame: usize,
    depth: u32,
}

impl Scope {
    /// Returns the scope for a child node: bound in `frame`, one level deeper.
    fn child(self, frame: usize) -> Self {
        Self {
            frame,
            depth: self.depth + 1,
        }
    }
}

/// Visitor context that maintains the environment stack used to resolve type
/// arguments.
pub struct VisitorCtx {
    env: Vec<EnvFrame>,
}

impl VisitorCtx {
    /// Resolves the type parameter at `ty_param_idx` within `frame_idx` to its
    /// terminal type, returning that type together with the frame its own free
    /// type parameters are bound in. A type argument is interpreted in its
    /// frame's `parent`, so chasing a parameter follows `parent` links (not
    /// stack position) — this is what makes a generic nominal used as another
    /// nominal's type argument resolve correctly. Results are cached per frame
    /// (union-find path compression).
    ///
    /// When the type parameter index is out of bounds, returns it wrapped in an
    /// error.
    fn resolve(
        &mut self,
        frame_idx: usize,
        ty_param_idx: usize,
    ) -> Result<(InternedType, usize), u16> {
        if let Some(hit) = self.env[frame_idx]
            .cache
            .get(ty_param_idx)
            .ok_or(ty_param_idx as u16)?
        {
            return Ok(*hit);
        }

        let next = *view_type_list(self.env[frame_idx].ty_args)
            .get(ty_param_idx)
            .ok_or(ty_param_idx as u16)?;

        let parent = self.env[frame_idx].parent;
        let resolved = if let Type::TypeParam { idx } = view_type(next) {
            // The root frame is its own parent; a type parameter there is
            // unbound.
            if parent == frame_idx {
                return Err(*idx);
            }
            self.resolve(parent, *idx as usize)?
        } else {
            // `next` is a literal argument of this frame, so its free type
            // parameters are bound in this frame's parent.
            (next, parent)
        };

        self.env[frame_idx].cache[ty_param_idx] = Some(resolved);
        Ok(resolved)
    }
}

fn visit<V: TypeVisitor>(
    visitor: &mut V,
    ctx: &mut VisitorCtx,
    scope: Scope,
    ty: InternedType,
) -> Result<(), VisitError<V::Error>> {
    if scope.depth > MAX_TRAVERSAL_DEPTH {
        return Err(VisitError::DepthExceeded);
    }

    // Resolve a type parameter to its terminal type and the frame its own free
    // type parameters are bound in; a literal type is bound in the current
    // frame. Children are then visited under that binding frame, one level
    // deeper.
    let (ty, ty_frame) = if let Type::TypeParam { idx } = view_type(ty) {
        ctx.resolve(scope.frame, *idx as usize)
            .map_err(|idx| VisitError::UnboundTypeParam { idx })?
    } else {
        (ty, scope.frame)
    };

    match view_type(ty) {
        Type::Bool => visitor.visit_bool().map_err(VisitError::Custom),
        Type::U8 => visitor.visit_u8().map_err(VisitError::Custom),
        Type::U16 => visitor.visit_u16().map_err(VisitError::Custom),
        Type::U32 => visitor.visit_u32().map_err(VisitError::Custom),
        Type::U64 => visitor.visit_u64().map_err(VisitError::Custom),
        Type::U128 => visitor.visit_u128().map_err(VisitError::Custom),
        Type::U256 => visitor.visit_u256().map_err(VisitError::Custom),
        Type::I8 => visitor.visit_i8().map_err(VisitError::Custom),
        Type::I16 => visitor.visit_i16().map_err(VisitError::Custom),
        Type::I32 => visitor.visit_i32().map_err(VisitError::Custom),
        Type::I64 => visitor.visit_i64().map_err(VisitError::Custom),
        Type::I128 => visitor.visit_i128().map_err(VisitError::Custom),
        Type::I256 => visitor.visit_i256().map_err(VisitError::Custom),
        Type::Address => visitor.visit_address().map_err(VisitError::Custom),
        Type::Signer => visitor.visit_signer().map_err(VisitError::Custom),
        Type::Vector { elem } => visitor.visit_vector(ctx, scope.child(ty_frame), *elem),
        Type::ImmutRef { inner } => visitor.visit_immut_ref(ctx, scope.child(ty_frame), *inner),
        Type::MutRef { inner } => visitor.visit_mut_ref(ctx, scope.child(ty_frame), *inner),
        Type::Function {
            args,
            results,
            abilities,
        } => visitor.visit_function(ctx, scope.child(ty_frame), *args, *results, *abilities),
        Type::Nominal {
            module_id,
            name,
            ty_args,
            ..
        } => {
            // The nominal's type arguments are bound in `ty_frame`; open a new
            // scope whose parent is that frame and descend against it.
            let needs_new_scope = !ty_args.is_empty();
            let frame = if needs_new_scope {
                ctx.env.push(EnvFrame::new(*ty_args, ty_frame));
                ctx.env.len() - 1
            } else {
                ty_frame
            };
            let result =
                visitor.visit_nominal(ctx, scope.child(frame), *module_id, *name, *ty_args);
            if needs_new_scope {
                ctx.env.pop();
            }
            result
        },
        Type::TypeParam { .. } => {
            unreachable!("type parameters are resolved before matching")
        },
    }
}

/// Walks type instantiated with the given type arguments, applying the visitor
/// to every type node.
pub fn walk_type<V: TypeVisitor>(
    ty: InternedType,
    ty_args: InternedTypeList,
    visitor: &mut V,
) -> Result<(), VisitError<V::Error>> {
    // The root frame is its own parent: its arguments must be concrete.
    let mut ctx = VisitorCtx {
        env: vec![EnvFrame::new(ty_args, 0)],
    };
    let root = Scope { frame: 0, depth: 0 };
    visit(visitor, &mut ctx, root, ty)
}

/// Errors produced by [`ConstSerializedSize`].
#[derive(Debug)]
pub enum ConstSerializedSizeError {
    /// The type is not constant-size: a vector, signer, reference, function, or
    /// enum appears somewhere within it.
    NonConstant,
    /// The provider had no entry for a nominal type encountered in a field.
    NominalNotFound,
}

/// Computes the constant BCS-serialized size of a fully substituted type, or
/// fails with [`ConstSerializedSizeError`]. Descends into struct fields via the provider;
/// type parameters in field types resolve lazily through the walker's env
/// stack.
pub struct ConstSerializedSize<'a, P> {
    provider: &'a P,
    total: u64,
}

/// Supplies a nominal's generic struct field types to a visitor that descends
/// into them (mirrors `PreparedModule::interned_field_types`). Production impls
/// look the module up and return its struct fields; an enum or unknown nominal
/// yields `None`.
pub trait NominalFieldProvider {
    fn struct_field_types(
        &self,
        module_id: InternedModuleId,
        name: InternedIdentifier,
    ) -> Option<FieldTypes>;
}

impl<'a, P: NominalFieldProvider> ConstSerializedSize<'a, P> {
    pub fn new(provider: &'a P) -> Self {
        Self { provider, total: 0 }
    }

    /// Walks `ty` instantiated with `ty_args` and returns its constant
    /// serialized size, or `None` if the type is not constant-size. A walk
    /// error (depth or an unbound type parameter) propagates as `Err`.
    pub fn size_of(
        provider: &'a P,
        ty: InternedType,
        ty_args: InternedTypeList,
    ) -> Result<Option<u64>, VisitError<ConstSerializedSizeError>> {
        let mut visitor = Self::new(provider);
        match walk_type(ty, ty_args, &mut visitor) {
            Ok(()) => Ok(Some(visitor.total)),
            Err(VisitError::Custom(ConstSerializedSizeError::NonConstant)) => Ok(None),
            Err(err @ VisitError::Custom(ConstSerializedSizeError::NominalNotFound))
            | Err(err @ VisitError::DepthExceeded)
            | Err(err @ VisitError::UnboundTypeParam { .. }) => Err(err),
        }
    }

    fn add(&mut self, num_bytes: u64) {
        self.total = self.total.saturating_add(num_bytes);
    }
}

impl<P: NominalFieldProvider> TypeVisitor for ConstSerializedSize<'_, P> {
    type Error = ConstSerializedSizeError;

    fn visit_bool(&mut self) -> Result<(), ConstSerializedSizeError> {
        self.add(1);
        Ok(())
    }

    fn visit_u8(&mut self) -> Result<(), ConstSerializedSizeError> {
        self.add(1);
        Ok(())
    }

    fn visit_u16(&mut self) -> Result<(), ConstSerializedSizeError> {
        self.add(2);
        Ok(())
    }

    fn visit_u32(&mut self) -> Result<(), ConstSerializedSizeError> {
        self.add(4);
        Ok(())
    }

    fn visit_u64(&mut self) -> Result<(), ConstSerializedSizeError> {
        self.add(8);
        Ok(())
    }

    fn visit_u128(&mut self) -> Result<(), ConstSerializedSizeError> {
        self.add(16);
        Ok(())
    }

    fn visit_u256(&mut self) -> Result<(), ConstSerializedSizeError> {
        self.add(32);
        Ok(())
    }

    fn visit_i8(&mut self) -> Result<(), ConstSerializedSizeError> {
        self.add(1);
        Ok(())
    }

    fn visit_i16(&mut self) -> Result<(), ConstSerializedSizeError> {
        self.add(2);
        Ok(())
    }

    fn visit_i32(&mut self) -> Result<(), ConstSerializedSizeError> {
        self.add(4);
        Ok(())
    }

    fn visit_i64(&mut self) -> Result<(), ConstSerializedSizeError> {
        self.add(8);
        Ok(())
    }

    fn visit_i128(&mut self) -> Result<(), ConstSerializedSizeError> {
        self.add(16);
        Ok(())
    }

    fn visit_i256(&mut self) -> Result<(), ConstSerializedSizeError> {
        self.add(32);
        Ok(())
    }

    fn visit_address(&mut self) -> Result<(), ConstSerializedSizeError> {
        self.add(32);
        Ok(())
    }

    fn visit_signer(&mut self) -> Result<(), ConstSerializedSizeError> {
        Err(ConstSerializedSizeError::NonConstant)
    }

    fn visit_vector(
        &mut self,
        _ctx: &mut VisitorCtx,
        _scope: Scope,
        _elem: InternedType,
    ) -> Result<(), VisitError<ConstSerializedSizeError>> {
        Err(VisitError::Custom(ConstSerializedSizeError::NonConstant))
    }

    fn visit_immut_ref(
        &mut self,
        _ctx: &mut VisitorCtx,
        _scope: Scope,
        _inner: InternedType,
    ) -> Result<(), VisitError<ConstSerializedSizeError>> {
        Err(VisitError::Custom(ConstSerializedSizeError::NonConstant))
    }

    fn visit_mut_ref(
        &mut self,
        _ctx: &mut VisitorCtx,
        _scope: Scope,
        _inner: InternedType,
    ) -> Result<(), VisitError<ConstSerializedSizeError>> {
        Err(VisitError::Custom(ConstSerializedSizeError::NonConstant))
    }

    fn visit_function(
        &mut self,
        _ctx: &mut VisitorCtx,
        _scope: Scope,
        _args: InternedTypeList,
        _results: InternedTypeList,
        _abilities: AbilitySet,
    ) -> Result<(), VisitError<ConstSerializedSizeError>> {
        Err(VisitError::Custom(ConstSerializedSizeError::NonConstant))
    }

    fn visit_nominal(
        &mut self,
        ctx: &mut VisitorCtx,
        scope: Scope,
        module_id: InternedModuleId,
        name: InternedIdentifier,
        _ty_args: InternedTypeList,
    ) -> Result<(), VisitError<ConstSerializedSizeError>> {
        match self.provider.struct_field_types(module_id, name) {
            // Only structs are constant-size; descend into every field.
            Some(FieldTypes::Struct(fields)) => {
                for field in fields {
                    visit(self, ctx, scope, field)?;
                }
                Ok(())
            },
            Some(FieldTypes::Enum(_)) => {
                Err(VisitError::Custom(ConstSerializedSizeError::NonConstant))
            },
            None => Err(VisitError::Custom(
                ConstSerializedSizeError::NominalNotFound,
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mono_move_core::{
        types::{ADDRESS_TY, BOOL_TY, EMPTY_TYPE_LIST, SIGNER_TY, U16_TY, U64_TY, U8_TY},
        Interner,
    };
    use mono_move_global_context::GlobalContext;
    use move_core_types::{account_address::AccountAddress, ident_str};
    use std::collections::HashMap;

    /// Records the visit order and descends into every registered struct field,
    /// so tests can assert traversal order through generics, vectors, refs, and
    /// functions.
    #[derive(Default)]
    struct TraceVisitor {
        events: Vec<&'static str>,
        fields: HashMap<InternedIdentifier, Vec<InternedType>>,
    }

    impl TraceVisitor {
        fn leaf(&mut self, event: &'static str) -> Result<(), ()> {
            self.events.push(event);
            Ok(())
        }
    }

    impl TypeVisitor for TraceVisitor {
        type Error = ();

        fn visit_bool(&mut self) -> Result<(), ()> {
            self.leaf("bool")
        }

        fn visit_u8(&mut self) -> Result<(), ()> {
            self.leaf("u8")
        }

        fn visit_u16(&mut self) -> Result<(), ()> {
            self.leaf("u16")
        }

        fn visit_u32(&mut self) -> Result<(), ()> {
            self.leaf("u32")
        }

        fn visit_u64(&mut self) -> Result<(), ()> {
            self.leaf("u64")
        }

        fn visit_u128(&mut self) -> Result<(), ()> {
            self.leaf("u128")
        }

        fn visit_u256(&mut self) -> Result<(), ()> {
            self.leaf("u256")
        }

        fn visit_i8(&mut self) -> Result<(), ()> {
            self.leaf("i8")
        }

        fn visit_i16(&mut self) -> Result<(), ()> {
            self.leaf("i16")
        }

        fn visit_i32(&mut self) -> Result<(), ()> {
            self.leaf("i32")
        }

        fn visit_i64(&mut self) -> Result<(), ()> {
            self.leaf("i64")
        }

        fn visit_i128(&mut self) -> Result<(), ()> {
            self.leaf("i128")
        }

        fn visit_i256(&mut self) -> Result<(), ()> {
            self.leaf("i256")
        }

        fn visit_address(&mut self) -> Result<(), ()> {
            self.leaf("address")
        }

        fn visit_signer(&mut self) -> Result<(), ()> {
            self.leaf("signer")
        }

        fn visit_vector(
            &mut self,
            ctx: &mut VisitorCtx,
            scope: Scope,
            elem: InternedType,
        ) -> Result<(), VisitError<()>> {
            self.events.push("vec");
            visit(self, ctx, scope, elem)
        }

        fn visit_immut_ref(
            &mut self,
            ctx: &mut VisitorCtx,
            scope: Scope,
            inner: InternedType,
        ) -> Result<(), VisitError<()>> {
            self.events.push("ref");
            visit(self, ctx, scope, inner)
        }

        fn visit_mut_ref(
            &mut self,
            ctx: &mut VisitorCtx,
            scope: Scope,
            inner: InternedType,
        ) -> Result<(), VisitError<()>> {
            self.events.push("mut_ref");
            visit(self, ctx, scope, inner)
        }

        fn visit_function(
            &mut self,
            ctx: &mut VisitorCtx,
            scope: Scope,
            args: InternedTypeList,
            results: InternedTypeList,
            _abilities: AbilitySet,
        ) -> Result<(), VisitError<()>> {
            self.events.push("fn");
            for arg in view_type_list(args) {
                visit(self, ctx, scope, *arg)?;
            }
            for res in view_type_list(results) {
                visit(self, ctx, scope, *res)?;
            }
            Ok(())
        }

        fn visit_nominal(
            &mut self,
            ctx: &mut VisitorCtx,
            scope: Scope,
            _module_id: InternedModuleId,
            name: InternedIdentifier,
            _ty_args: InternedTypeList,
        ) -> Result<(), VisitError<()>> {
            self.events.push("struct");
            // Copy out before recursing so the borrow of `self.fields` ends.
            let fields = self.fields.get(&name).cloned().unwrap_or_default();
            for field in fields {
                visit(self, ctx, scope, field)?;
            }
            Ok(())
        }
    }

    /// Maps a nominal name to its field layout.
    #[derive(Default)]
    struct FakeProvider {
        fields: HashMap<InternedIdentifier, FieldTypes>,
    }

    impl NominalFieldProvider for FakeProvider {
        fn struct_field_types(
            &self,
            _module_id: InternedModuleId,
            name: InternedIdentifier,
        ) -> Option<FieldTypes> {
            self.fields.get(&name).cloned()
        }
    }

    #[test]
    fn trace_resolves_generic_struct_fields() {
        // struct S<T> { vector<T>, T } at [u64]: the field type parameters
        // resolve to u64 across the vector and directly.
        let gc = GlobalContext::with_num_execution_workers(1);
        let g = gc.try_execution_context(0).unwrap();
        let mid = g.module_id_of(&AccountAddress::ONE, ident_str!("m"));
        let s = g.identifier_of(ident_str!("S"));
        let t0 = g.type_param_of(0);

        let mut v = TraceVisitor::default();
        v.fields.insert(s, vec![g.vector_of(t0), t0]);

        let s_u64 = g.nominal_of(mid, s, g.type_list_of(&[U64_TY]));
        walk_type(s_u64, EMPTY_TYPE_LIST, &mut v).unwrap();
        assert_eq!(v.events, ["struct", "vec", "u64", "u64"]);
    }

    #[test]
    fn trace_chases_type_param_across_frames() {
        // struct O<T> { I<T> }, struct I<T> { T }, walked as O<u64>: the inner
        // field chases I's frame -> O's frame -> u64.
        let gc = GlobalContext::with_num_execution_workers(1);
        let g = gc.try_execution_context(0).unwrap();
        let mid = g.module_id_of(&AccountAddress::ONE, ident_str!("m"));
        let o = g.identifier_of(ident_str!("O"));
        let i = g.identifier_of(ident_str!("I"));
        let t0 = g.type_param_of(0);

        let mut v = TraceVisitor::default();
        v.fields
            .insert(o, vec![g.nominal_of(mid, i, g.type_list_of(&[t0]))]);
        v.fields.insert(i, vec![t0]);

        let o_u64 = g.nominal_of(mid, o, g.type_list_of(&[U64_TY]));
        walk_type(o_u64, EMPTY_TYPE_LIST, &mut v).unwrap();
        assert_eq!(v.events, ["struct", "struct", "u64"]);
    }

    #[test]
    fn trace_recurses_references_and_functions() {
        let gc = GlobalContext::with_num_execution_workers(1);
        let g = gc.try_execution_context(0).unwrap();

        let mut v = TraceVisitor::default();
        walk_type(g.immut_ref_of(U64_TY), EMPTY_TYPE_LIST, &mut v).unwrap();
        assert_eq!(v.events, ["ref", "u64"]);

        let func = g.function_of(
            g.type_list_of(&[U64_TY]),
            g.type_list_of(&[BOOL_TY]),
            AbilitySet::EMPTY,
        );
        let mut v = TraceVisitor::default();
        walk_type(func, EMPTY_TYPE_LIST, &mut v).unwrap();
        assert_eq!(v.events, ["fn", "u64", "bool"]);
    }

    #[test]
    fn const_size_primitive_struct() {
        // struct Pair { u64, bool } -> 9.
        let gc = GlobalContext::with_num_execution_workers(1);
        let g = gc.try_execution_context(0).unwrap();
        let mid = g.module_id_of(&AccountAddress::ONE, ident_str!("m"));
        let pair = g.identifier_of(ident_str!("Pair"));

        let mut p = FakeProvider::default();
        p.fields
            .insert(pair, FieldTypes::Struct(vec![U64_TY, BOOL_TY]));

        let ty = g.nominal_of(mid, pair, EMPTY_TYPE_LIST);
        assert_eq!(
            ConstSerializedSize::size_of(&p, ty, EMPTY_TYPE_LIST).unwrap(),
            Some(9)
        );
    }

    #[test]
    fn const_size_nominal_as_type_argument() {
        // struct S<Y> { Y }, struct Box<X> { X }, struct Outer<T> { Box<S<T>> }.
        // Outer<u64> = Box<S<u64>> = S<u64> = u64 -> 8.
        let gc = GlobalContext::with_num_execution_workers(1);
        let g = gc.try_execution_context(0).unwrap();
        let mid = g.module_id_of(&AccountAddress::ONE, ident_str!("m"));
        let s = g.identifier_of(ident_str!("S"));
        let boxed = g.identifier_of(ident_str!("Box"));
        let outer = g.identifier_of(ident_str!("Outer"));
        let t0 = g.type_param_of(0);

        let mut p = FakeProvider::default();
        p.fields.insert(s, FieldTypes::Struct(vec![t0]));
        p.fields.insert(boxed, FieldTypes::Struct(vec![t0]));
        let s_t0 = g.nominal_of(mid, s, g.type_list_of(&[t0]));
        let box_s_t0 = g.nominal_of(mid, boxed, g.type_list_of(&[s_t0]));
        p.fields.insert(outer, FieldTypes::Struct(vec![box_s_t0]));

        let outer_u64 = g.nominal_of(mid, outer, g.type_list_of(&[U64_TY]));
        assert_eq!(
            ConstSerializedSize::size_of(&p, outer_u64, EMPTY_TYPE_LIST).unwrap(),
            Some(8)
        );
    }

    #[test]
    fn const_size_cross_frame_generics() {
        // struct Inner<T> { T, T }
        // struct Outer<A, B> { Inner<A>, B, u8 }
        // Outer<u16, address> -> Inner<u16>(2+2) + address(32) + u8(1) = 37.
        let gc = GlobalContext::with_num_execution_workers(1);
        let g = gc.try_execution_context(0).unwrap();
        let mid = g.module_id_of(&AccountAddress::ONE, ident_str!("m"));
        let inner = g.identifier_of(ident_str!("Inner"));
        let outer = g.identifier_of(ident_str!("Outer"));
        let t0 = g.type_param_of(0);
        let t1 = g.type_param_of(1);

        let mut p = FakeProvider::default();
        p.fields.insert(inner, FieldTypes::Struct(vec![t0, t0]));
        p.fields.insert(
            outer,
            FieldTypes::Struct(vec![
                g.nominal_of(mid, inner, g.type_list_of(&[t0])),
                t1,
                U8_TY,
            ]),
        );

        let ty = g.nominal_of(mid, outer, g.type_list_of(&[U16_TY, ADDRESS_TY]));
        assert_eq!(
            ConstSerializedSize::size_of(&p, ty, EMPTY_TYPE_LIST).unwrap(),
            Some(37)
        );
    }

    #[test]
    fn const_size_type_param_instantiated_with_struct() {
        // struct Box<T> { T }, struct Pair { u64, bool }; Box<Pair> -> 9.
        let gc = GlobalContext::with_num_execution_workers(1);
        let g = gc.try_execution_context(0).unwrap();
        let mid = g.module_id_of(&AccountAddress::ONE, ident_str!("m"));
        let boxed = g.identifier_of(ident_str!("Box"));
        let pair = g.identifier_of(ident_str!("Pair"));
        let t0 = g.type_param_of(0);

        let mut p = FakeProvider::default();
        p.fields.insert(boxed, FieldTypes::Struct(vec![t0]));
        p.fields
            .insert(pair, FieldTypes::Struct(vec![U64_TY, BOOL_TY]));

        let pair_ty = g.nominal_of(mid, pair, EMPTY_TYPE_LIST);
        let box_pair = g.nominal_of(mid, boxed, g.type_list_of(&[pair_ty]));
        assert_eq!(
            ConstSerializedSize::size_of(&p, box_pair, EMPTY_TYPE_LIST).unwrap(),
            Some(9)
        );
    }

    #[test]
    fn const_size_type_param_instantiated_with_vector_is_none() {
        // Box<vector<u8>>: the field resolves to a vector, which is not
        // constant-size.
        let gc = GlobalContext::with_num_execution_workers(1);
        let g = gc.try_execution_context(0).unwrap();
        let mid = g.module_id_of(&AccountAddress::ONE, ident_str!("m"));
        let boxed = g.identifier_of(ident_str!("Box"));
        let t0 = g.type_param_of(0);

        let mut p = FakeProvider::default();
        p.fields.insert(boxed, FieldTypes::Struct(vec![t0]));

        let box_vec = g.nominal_of(mid, boxed, g.type_list_of(&[g.vector_of(U8_TY)]));
        assert_eq!(
            ConstSerializedSize::size_of(&p, box_vec, EMPTY_TYPE_LIST).unwrap(),
            None
        );
    }

    #[test]
    fn const_size_non_constant_fields_are_none() {
        let gc = GlobalContext::with_num_execution_workers(1);
        let g = gc.try_execution_context(0).unwrap();
        let mid = g.module_id_of(&AccountAddress::ONE, ident_str!("m"));

        let has_vec = g.identifier_of(ident_str!("HasVec"));
        let has_signer = g.identifier_of(ident_str!("HasSigner"));
        let an_enum = g.identifier_of(ident_str!("AnEnum"));

        let mut p = FakeProvider::default();
        p.fields
            .insert(has_vec, FieldTypes::Struct(vec![g.vector_of(U8_TY)]));
        p.fields
            .insert(has_signer, FieldTypes::Struct(vec![SIGNER_TY]));
        p.fields
            .insert(an_enum, FieldTypes::Enum(vec![vec![U64_TY]]));

        for name in [has_vec, has_signer, an_enum] {
            let ty = g.nominal_of(mid, name, EMPTY_TYPE_LIST);
            assert_eq!(
                ConstSerializedSize::size_of(&p, ty, EMPTY_TYPE_LIST).unwrap(),
                None
            );
        }
    }

    #[test]
    fn unknown_nominal_errors() {
        let gc = GlobalContext::with_num_execution_workers(1);
        let g = gc.try_execution_context(0).unwrap();
        let mid = g.module_id_of(&AccountAddress::ONE, ident_str!("m"));
        let ghost = g.identifier_of(ident_str!("Ghost"));

        let p = FakeProvider::default();
        let ty = g.nominal_of(mid, ghost, EMPTY_TYPE_LIST);
        assert!(matches!(
            ConstSerializedSize::size_of(&p, ty, EMPTY_TYPE_LIST),
            Err(VisitError::Custom(
                ConstSerializedSizeError::NominalNotFound
            ))
        ));
    }

    #[test]
    fn unbound_type_param_errors() {
        let gc = GlobalContext::with_num_execution_workers(1);
        let g = gc.try_execution_context(0).unwrap();
        let p = FakeProvider::default();
        assert!(matches!(
            ConstSerializedSize::size_of(&p, g.type_param_of(0), EMPTY_TYPE_LIST),
            Err(VisitError::UnboundTypeParam { idx: 0 })
        ));
    }

    #[test]
    fn recursive_struct_hits_depth_limit() {
        // struct Loop { Loop } recurses without bound -> DepthExceeded.
        let gc = GlobalContext::with_num_execution_workers(1);
        let g = gc.try_execution_context(0).unwrap();
        let mid = g.module_id_of(&AccountAddress::ONE, ident_str!("m"));
        let looping = g.identifier_of(ident_str!("Loop"));

        let mut p = FakeProvider::default();
        let loop_ty = g.nominal_of(mid, looping, EMPTY_TYPE_LIST);
        p.fields.insert(looping, FieldTypes::Struct(vec![loop_ty]));

        assert!(matches!(
            ConstSerializedSize::size_of(&p, loop_ty, EMPTY_TYPE_LIST),
            Err(VisitError::DepthExceeded)
        ));
    }
}
