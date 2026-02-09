# Type Interning Implementation Plan

## Objective

Implement thread-safe interning for Move types:
- `SignatureToken` and `&[SignatureToken]` without TypeParameter
- `TypeTag` and `&[TypeTag]`
in the global-context crate, enabling deduplication and O(1) equality checks through pointer identity

## Design Overview

### Core Principles

1. **Canonical Representation**: All equivalent types (whether from SignatureToken or TypeTag) intern to the same pointer
2. **Bottom-up Construction**: Intern leaf types first, then compose into compound types
3. **Two Interners**: Separate interners for `Type` and `TypeList` to maximize deduplication
4. **Identity Equality**: Pointer comparison (O(1)) instead of structural comparison
5. **Arena Allocation**: All Type/TypeList instances allocated in arena for stable pointers
6. **Zero-Clone Lookups**: Borrowed keys with inline struct resolution to avoid cloning on cache hits

### Type Representation

```rust
// Internal type representation
pub(crate) enum Type {
    // Primitives
    Bool,
    U8, U16, U32, U64, U128, U256,
    I8, I16, I32, I64, I128, I256,
    Address,
    Signer,

    // Vectors, references.
    Vector(ArenaPtr<Type>),
    Reference(ArenaPtr<Type>),
    ReferenceMut(ArenaPtr<Type>),

    // Structs.
    Struct {
        // Module ID, struct name an type arguments.
        id: ArenaPtr<ExecutableId<'static>>,
        name: ArenaPtr<str>,
        type_args: ArenaPtr<TypeList>,
    },

    // Function types.
    Function {
        args: ArenaPtr<TypeList>,
        results: ArenaPtr<TypeList>,
        abilities: AbilitySet,
    },
}

// Internal type list representation - stores slice pointer
pub(crate) struct TypeList(*const [ArenaPtr<Type>]);

// Public API - lifetime-bound pointers
pub struct TypePtr<'a>(&'a Type);
pub struct TypeListPtr<'a>(&'a TypeList);
```

### Canonical Key Design for Interner

Keys for interner's DashMap lookups that enable zero-clone queries:

```rust
// Key stored in DashMap<TypeKey, ArenaPtr<Type>>, key is a pointer to created allocation.
struct TypeKey<'a>(&'a Type);

// Borrowed key for TypeTag lookups - zero clone
struct TypeTagKey<'a>(&'a TypeTag);

// Borrowed key for SignatureToken lookups - includes resolution context
struct SignatureTokenKey<'a>(&'a SignatureToken, &'a CompiledModule);

// Key stored in DashMap<TypeListKey, ArenaPtr<TypeList>>, key is a pointer to created allocation.
struct TypeListKey<'a>(&'a TypeList);

// Borrowed key for TypeTag list lookups - zero clone
struct TypeTagListKey<'a>(&'a [TypeTag]);

// Borrowed key for SignatureToken list lookups - includes resolution context
struct SignatureTokenListKey<'a>(&'a [SignatureToken], &'a CompiledModule);
```

**Key Insights**:

1. Both `SignatureTokenKey` and `SignatureTokenListKey` include `CompiledModule` for inline struct resolution during hashing and equality checks, ensuring cross-format deduplication works correctly.
2. `TypeKey`, `TypeTagKey`, `SignatureTokenKey` have same hashing and implement equivalence between each other.
3. `TypeListKey`, `TypeTagListKey`, `SignatureTokenListKey` have same hashing and implement equivalence between each other.

### Interning Algorithm

**For TypeTag**:

1. Create TypeTagKey(&type_tag) for lookup
2. Check DashMap - if found, return ArenaPtr (zero clone!)
3. On miss: recursively intern children (for compound types)
4. Build Type with interned children
5. Insert the type pointer in the map with key TypeKey(&'static Type)

**For SignatureToken**:

1. Create SignatureTokenKey(&token, &module) for lookup (panics if TypeParameter)
2. Check DashMap - if found, return ArenaPtr (zero clone, resolution happens in hash/eq!)
3. On miss: recursively intern children
4. For Struct/StructInstantiation: resolve to address/module/name using existing identifier interner
5. Build Type with interned children and ExecutableId
6. Insert the type pointer in the map with key TypeKey(&'static Type)

**For TypeTag List**:

1. Create TypeTagListKey(&[TypeTag]) for lookup
2. Check DashMap - if found, return ArenaPtr<TypeList> (zero clone!)
3. On miss: intern each TypeTag to get Vec<ArenaPtr<Type>>
4. Allocate slice in arena and wrap in TypeList
5. Insert the type list pointer in the map with key TypeListKey(&'static TypeList)

**For SignatureToken List**:

1. Create SignatureTokenListKey(&[SignatureToken], &module) for lookup
2. Check DashMap - if found, return ArenaPtr<TypeList> (zero clone, resolution happens in hash/eq!)
3. On miss: intern each SignatureToken to get Vec<ArenaPtr<Type>>
4. Allocate slice in arena and wrap in TypeList
5. Insert the type list pointer in the map with key TypeListKey(&'static TypeList)

### Hash and Equality Strategy

**Critical Requirements**:
- `SignatureToken::U64` must hash same as `TypeTag::U64` and same as `TypeKey(&'static Type::U64)`, and so on.
- `SignatureToken::Struct(idx)` must hash same as `TypeTag::Struct(...)` when they refer to the same type
- `Vec<SignatureToken>` must hash same as `Vec<TypeTag>` when they refer to the same types, and same as `TypeListKey(...)`
- Resolution happens inline during hash/equivalent checks
- **Zero clones in `Equivalent` implementations**

**Implementation**:

```rust
// Canonical discriminants for cross-format hashing
const DISC_BOOL: u8 = 0;
const DISC_U8: u8 = 1;
const DISC_U64: u8 = 2;
const DISC_U128: u8 = 3;
const DISC_U256: u8 = 4;
const DISC_ADDRESS: u8 = 5;
const DISC_SIGNER: u8 = 6;
const DISC_VECTOR: u8 = 7;
const DISC_STRUCT: u8 = 8;
const DISC_REFERENCE: u8 = 9;
const DISC_REFERENCE_MUT: u8 = 10;
const DISC_FUNCTION: u8 = 11;
// ... etc for all variants

impl Hash for TypeTagKey<'_> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match self.0 {
            TypeTag::Bool => DISC_BOOL.hash(state),
            TypeTag::U8 => DISC_U8.hash(state),
            TypeTag::U64 => DISC_U64.hash(state),

            TypeTag::Vector(inner) => {
                DISC_VECTOR.hash(state);
                TypeTagKey(inner.as_ref()).hash(state);
            }

            TypeTag::Struct(StructTag { address, module, name, type_args }) => {
                DISC_STRUCT.hash(state);
                address.hash(state);
                module.as_ident_str().as_str().hash(state);
                name.as_ident_str().as_str().hash(state);
                type_args.len().hash(state);
                for arg in type_args {
                    TypeTagKey(arg).hash(state);
                }
            }
            // ... other variants
        }
    }
}

impl Hash for SignatureTokenKey<'_> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        let (token, module) = (self.0, self.1);

        match token {
            SignatureToken::Bool => DISC_BOOL.hash(state),
            SignatureToken::U8 => DISC_U8.hash(state),
            SignatureToken::U64 => DISC_U64.hash(state),

            SignatureToken::Vector(inner) => {
                DISC_VECTOR.hash(state);
                SignatureTokenKey(inner.as_ref(), module).hash(state);
            }

            SignatureToken::Reference(inner) => {
                DISC_REFERENCE.hash(state);
                SignatureTokenKey(inner.as_ref(), module).hash(state);
            }

            SignatureToken::MutableReference(inner) => {
                DISC_REFERENCE_MUT.hash(state);
                SignatureTokenKey(inner.as_ref(), module).hash(state);
            }

            // Resolve struct inline during hashing!
            SignatureToken::Struct(idx) => {
                DISC_STRUCT.hash(state);

                let handle = module.struct_handle_at(*idx);
                let module_handle = module.module_handle_at(handle.module);

                // Hash resolved address/module/name
                module_handle.address.hash(state);
                module.identifier_at(module_handle.name).as_str().hash(state);
                module.identifier_at(handle.name).as_str().hash(state);
                0usize.hash(state);  // Empty type args
            }

            SignatureToken::StructInstantiation(idx, type_args) => {
                DISC_STRUCT.hash(state);

                let handle = module.struct_handle_at(*idx);
                let module_handle = module.module_handle_at(handle.module);

                // Hash resolved address/module/name
                module_handle.address.hash(state);
                module.identifier_at(module_handle.name).as_str().hash(state);
                module.identifier_at(handle.name).as_str().hash(state);

                // Hash type args
                type_args.len().hash(state);
                for arg in type_args {
                    SignatureTokenKey(arg, module).hash(state);
                }
            }

            SignatureToken::TypeParameter(_) => {
                panic!("TypeParameter cannot be interned - caller must check for TypeParameter before calling intern functions")
            }

            // ... other variants
        }
    }
}

impl Hash for TypeKey<'_> {
   // TODO: implement structural hash.
}

impl Equivalent<TypeKey<'_>> for TypeTagKey<'_> {
    fn equivalent(&self, key: &TypeKey) -> bool {
       // TODO: implement structural equality.
    }
}

impl Equivalent<TypeKey<'_>> for SignatureTokenKey<'_> {
    fn equivalent(&self, key: &TypeKey) -> bool {
       // TODO: implement structural equality.
    }
}

impl Hash for TypeTagListKey<'_> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.len().hash(state);
        for tag in self.0 {
            TypeTagKey(tag).hash(state);  // Reuse TypeTagKey hashing
        }
    }
}

impl Hash for SignatureTokenListKey<'_> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        let (tokens, module) = (self.0, self.1);
        tokens.len().hash(state);
        for token in tokens {
            SignatureTokenKey(token, module).hash(state);  // Reuse SignatureTokenKey hashing
        }
    }
}

impl Equivalent<TypeListKey<'_>> for TypeTagListKey<'_> {
    fn equivalent(&self, key: &TypeListKey) -> bool {
       // TODO: implement structural equality.
    }
}

impl Equivalent<TypeListKey<'_>> for SignatureTokenListKey<'_> {
    fn equivalent(&self, key: &TypeListKey) -> bool {
       // TODO: implement structural equality.
    }
}
```

## Implementation Steps

### Step 1: Extend ArenaAllocator (arena.rs)

Add slice allocation support:

```rust
pub trait ArenaAllocator {
    fn alloc<T>(&self, value: T) -> ArenaPtr<T>;
    fn alloc_str(&self, s: &str) -> ArenaPtr<str>;
    fn allocated_bytes(&self) -> usize;

    // NEW: Allocate slice by copying
    fn alloc_slice_copy<T: Copy>(&self, src: &[T]) -> ArenaPtr<[T]>;
}

impl ArenaAllocator for ArenaGuard<'_> {
    // ... existing methods ...

    fn alloc_slice_copy<T: Copy>(&self, src: &[T]) -> ArenaPtr<[T]> {
        let slice_ref = self.guard.alloc_slice_copy(src);
        ArenaPtr(NonNull::from(slice_ref))
    }
}
```

**Files**: `src/arena.rs`

### Step 2: Define Internal Types (types.rs)

Add Type and TypeList definitions.

**Files**: `src/types.rs` (lines ~150+)

### Step 3: Define Public API (types.rs)

Add `TypePtr<'a>` and `TypeListPtr<'a>` with conversion methods and accessors.

**Files**: `src/types.rs` (lines ~200+)

### Step 4: Define Canonical Keys (new file: type_keys.rs)

Create `src/type_keys.rs` with:
- `TypeKey<'a>` struct
- `TypeTagKey<'a>` struct
- `SignatureTokenKey<'a>` struct (with CompiledModule)
- `TypeListKey<'a>` struct
- `TypeTagListKey<'a>` struct
- `SignatureTokenListKey<'a>` struct (with CompiledModule)
- Canonical discriminant constants
- Hash implementations
- Equivalent trait implementations

**Files**: `src/type_keys.rs` (new file)

Update `src/lib.rs`:
```rust
mod type_keys;
```

### Step 5: Add Type Interners to SharedContext (context.rs)

Update SharedContext:
```rust
pub(crate) struct SharedContext {
    // Existing fields...
    identifier_interner: DashMapInterner<String>,

    // NEW: Type interners
    type_interner: DashMap<TypeKey<'static>, ArenaPtr<Type>>,
    type_list_interner: DashMap<TypeListKey<'static>, ArenaPtr<TypeList>>,
}
```

**Files**: `src/context.rs` (lines ~53-63, ~134-139)

### Step 6: Implement Type Interning for TypeTag (context.rs)

Add methods to `ExecutionContext`:

```rust
fn intern_type_tag(&self, type_tag: &TypeTag) -> ArenaPtr<Type> {
    // TODO: implement similarly how intern_module_id is done.
}

fn intern_type_tags(&self, tags: &[TypeTag]) -> ArenaPtr<TypeList> {
   // TODO: implement similarly how intern_module_id is done.
}
```

**Files**: `src/context.rs` (lines ~327+)

### Step 7: Implement Type Interning for SignatureToken (context.rs)

Add `intern_signature_token` method to `ExecutionContext`:

```rust
/// Interns a SignatureToken and returns a stable pointer [`TypePtr`].
///
/// # Panics
///
/// Panics if the token is a TypeParameter. Caller must check for TypeParameter
/// before calling this function.
pub fn intern_signature_token<'b>(
    &'b self,
    token: &SignatureToken,
    module: &CompiledModule,
) -> TypePtr<'b>
where
    'a: 'b,
{
    let ty = self.intern_with(
        &SignatureTokenKey(token, module),
        &self.shared_guard.types,
        |arena| {
            // Build type recursively
            match token {
                SignatureToken::Bool => arena.alloc(Type::Bool),
                SignatureToken::U8 => arena.alloc(Type::U8),
                SignatureToken::U64 => arena.alloc(Type::U64),

                SignatureToken::Vector(inner) => {
                    let inner_ptr = self.intern_signature_token(inner, module);
                    arena.alloc(Type::Vector(inner_ptr.into_arena_ptr()))
                }

                SignatureToken::Reference(inner) => {
                    let inner_ptr = self.intern_signature_token(inner, module);
                    arena.alloc(Type::Reference(inner_ptr.into_arena_ptr()))
                }

                SignatureToken::MutableReference(inner) => {
                    let inner_ptr = self.intern_signature_token(inner, module);
                    arena.alloc(Type::ReferenceMut(inner_ptr.into_arena_ptr()))
                }

                SignatureToken::Struct(idx) => {
                    let handle = module.struct_handle_at(*idx);
                    let module_handle = module.module_handle_at(handle.module);

                    let module_id = self.intern_address_name(
                        &module_handle.address,
                        module.identifier_at(module_handle.name),
                    );
                    let struct_name = arena.alloc_str(module.identifier_at(handle.name).as_str());
                    let empty_type_list = self.intern_empty_type_list();

                    arena.alloc(Type::Struct {
                        id: module_id.into_arena_ptr(),
                        name: struct_name,
                        type_args: empty_type_list.into_arena_ptr(),
                    })
                }

                SignatureToken::StructInstantiation(idx, type_args) => {
                    let handle = module.struct_handle_at(*idx);
                    let module_handle = module.module_handle_at(handle.module);

                    let module_id = self.intern_address_name(
                        &module_handle.address,
                        module.identifier_at(module_handle.name),
                    );
                    let struct_name = arena.alloc_str(module.identifier_at(handle.name).as_str());
                    let type_args_ptr = self.intern_signature_tokens(type_args, module);

                    arena.alloc(Type::Struct {
                        id: module_id.into_arena_ptr(),
                        name: struct_name,
                        type_args: type_args_ptr.into_arena_ptr(),
                    })
                }

                SignatureToken::TypeParameter(_) => {
                    panic!("TypeParameter cannot be interned - caller must check before calling")
                }
            }
        },
        counters::log_type_interner_cache_miss,
    );

    TypePtr::new_internal(ty)
}

/// Interns a list of SignatureTokens and returns a stable pointer [`TypeListPtr`].
///
/// # Panics
///
/// Panics if any token is a TypeParameter.
pub fn intern_signature_tokens<'b>(
    &'b self,
    tokens: &[SignatureToken],
    module: &CompiledModule,
) -> TypeListPtr<'b>
where
    'a: 'b,
{
    let type_list = self.intern_with(
        &SignatureTokenListKey(tokens, module),
        &self.shared_guard.type_lists,
        |arena| {
            // Intern each type
            let interned_types: Vec<_> = tokens
                .iter()
                .map(|token| self.intern_signature_token(token, module).into_arena_ptr())
                .collect();

            // Allocate slice in arena
            let slice_ptr = arena.alloc_slice_copy(&interned_types);
            arena.alloc(TypeList(slice_ptr.as_ptr()))
        },
        counters::log_type_list_interner_cache_miss,
    );

    TypeListPtr::new_internal(type_list)
}

/// Helper to get an empty type list (cached to avoid repeated allocations).
fn intern_empty_type_list<'b>(&'b self) -> TypeListPtr<'b>
where
    'a: 'b,
{
    self.intern_type_tags(&[])
}
```

**Files**: `src/context.rs` (lines ~450+)

### Step 8: Add Prometheus Counters (counters.rs)

Add type interner cache miss and count metrics:

```rust
static TYPE_INTERNER_CACHE_MISS: Lazy<IntCounter> = ...;
static TYPE_LIST_INTERNER_CACHE_MISS: Lazy<IntCounter> = ...;
static INTERNED_TYPE_COUNT: Lazy<IntGauge> = ...;
static INTERNED_TYPE_LIST_COUNT: Lazy<IntGauge> = ...;
```

**Files**: `src/counters.rs` (lines ~86+)

### Step 9: Update MaintenanceContext (context.rs)

Add `interned_type_count()`, `interned_type_list_count()` and update `check_memory_usage()` to flush type interners.

**Files**: `src/context.rs` (lines ~350-400)

### Step 10: Testing

Create comprehensive test suite in `tests/type_interning_tests.rs` with tests for:
- Hashing and equivalence tests (proptests?)
- Primitive types
- Vector types (simple and nested)
- Struct types (generic and non-generic)
- Reference types (Reference and ReferenceMut)
- TypeList deduplication
- Empty type lists
- Parallel interning
- Cross-format deduplication (TypeTag and SignatureToken → same pointer)
- Cross-format deduplication for type lists (Vec<TypeTag> and Vec<SignatureToken> → same pointer)
- SignatureToken struct resolution
- TypeParameter panic
- Maintenance flush

**Files**: `tests/type_interning_tests.rs` (new file)

## Performance Optimizations

1. **Zero clones on cache hit**: Borrowed keys (TypeTagKey, SignatureTokenKey, TypeTagListKey, SignatureTokenListKey) for lookups
2. **Zero clones in Equivalent**: Helper functions perform direct recursive comparison without allocating intermediate keys
3. **Inline struct resolution**: Resolution happens during hash/equivalent checks in SignatureTokenKey and SignatureTokenListKey
4. **Single allocation for struct identity**: `ArenaPtr<ExecutableId<'static>>` instead of 3 separate pointers
5. **Reuse identifier interner**: Module/struct names deduplicated via existing identifier interner
6. **Bottom-up construction**: Intern children first, compose into parents (maximizes reuse)
7. **Empty list caching**: Special case for empty type lists (common)
8. **Separate Type/TypeList interners**: Independent deduplication for types and lists
9. **Cross-format list deduplication**: Vec<TypeTag> and Vec<SignatureToken> deduplicate to same TypeList

## Critical Correctness Considerations

1. **Cross-format hashing**: Canonical discriminants ensure TypeTag and SignatureToken hash identically
2. **Cross-format list hashing**: TypeTagListKey and SignatureTokenListKey hash identically for equivalent type lists
3. **Inline struct resolution**: SignatureTokenKey and SignatureTokenListKey resolve structs during hash/equivalent without materializing intermediate types
4. **Zero-clone comparison**: Helper functions avoid allocating TypeKey instances during Equivalent checks
5. **Cross-format asymmetry**: TypeTagKey cannot match structs in TypeKey::Token (no module context), but SignatureTokenKey can match all types in TypeKey::Tag (has module context)
6. **Pointer safety**: TypeList stores `*const [ArenaPtr<Type>]` valid until arena flush
7. **Empty type list**: Special case to avoid repeated empty slice allocations
8. **TypeParameter handling**: Panics on TypeParameter - caller must check before calling
9. **Reference handling**: Both `&T` and `&mut T` intern to different types (Reference vs ReferenceMut)

## Testing Checklist

- [ ] Primitive type interning and deduplication
- [ ] Vector type interning (simple and nested)
- [ ] Struct type interning (with and without type args)
- [ ] Reference and mutable reference types
- [ ] Function type interning
- [ ] TypeList deduplication
- [ ] Empty type list special case
- [ ] Parallel interning across workers
- [ ] Cross-format deduplication (TypeTag and SignatureToken → same pointer)
- [ ] Cross-format list deduplication (Vec<TypeTag> and Vec<SignatureToken> → same pointer)
- [ ] SignatureToken struct resolution with module context
- [ ] Identifier interner reuse for module/struct names
- [ ] TypeParameter panics
- [ ] Maintenance flush clears type interners
- [ ] Prometheus metrics correctness

## Final Notes

This implementation achieves:
- **Zero-clone lookups** via borrowed keys with inline resolution
- **Zero-clone comparisons** via helper functions that avoid intermediate allocations
- **Maximum deduplication** through canonical hashing across TypeTag/SignatureToken for both types and type lists
- **Minimal allocations** via ExecutableId single-pointer optimization and identifier interner reuse
- **O(1) equality** via pointer identity
- **Thread-safe** via DashMap-based interners
- **Performance-first** design with bottom-up construction

The design is rigorous, safe (all unsafe blocks documented), and efficient.
