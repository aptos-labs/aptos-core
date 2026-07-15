// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Object descriptors and the [`DescriptorProvider`] trait.
//!
//! An [`ObjectDescriptor`] tells the GC how to trace internal pointers
//! within a single heap object. Two entries are reserved at fixed
//! [`DescriptorId`] slots: `Trivial` (id `0`) and `Closure` (id `1`).
//! Producers (the global context, tests) append user descriptors starting
//! at [`RESERVED_DESCRIPTOR_COUNT`]; consumers (the runtime) look them up
//! through a [`DescriptorProvider`].

use crate::DescriptorId;

// ---------------------------------------------------------------------------
// Object descriptors (for GC tracing)
// ---------------------------------------------------------------------------

/// Describes the reference layout of a heap object so the GC knows how to
/// trace internal pointers. Only one level of indirection is described;
/// pointed-to objects are self-describing via their own headers.
///
/// External code creates descriptors through the validating constructors
/// ([`Self::new_vector`], [`Self::new_struct`], [`Self::new_enum`],
/// [`Self::new_captured_data`]) which return `anyhow::Result`. `Trivial`
/// and `Closure` aren't externally constructable; they live only at their
/// reserved slots via [`TRIVIAL_DESCRIPTOR`] / [`CLOSURE_DESCRIPTOR`].
/// Any `&ObjectDescriptor` flowing through the runtime is therefore
/// self-sound by construction (nonzero size, in-bounds 8-byte-aligned
/// strictly-sorted pointer offsets).
///
/// [`Self::inner`] exposes the internal variant for the runtime's GC and
/// verifier to dispatch on. The opacity guarantee is "external code can
/// only *construct* validated descriptors"; reading the variant is fine.
#[derive(Debug)]
pub struct ObjectDescriptor(ObjectDescriptorInner);

/// Variants of [`ObjectDescriptor`]. The runtime's GC and verifier match
/// on this via [`ObjectDescriptor::inner`].
#[derive(Debug)]
pub enum ObjectDescriptorInner {
    /// No internal heap references. GC copies the blob and moves on.
    Trivial,

    /// Closure object — fixed runtime layout shared by every closure.
    Closure,

    /// Vector whose elements may contain heap pointers at known offsets.
    Vector {
        /// Size of each element in bytes.
        /// The address of element `i` is `data_start + i * elem_size`.
        elem_size: u32,
        /// Byte offsets within each element that are heap pointers.
        elem_pointer_offsets: Vec<u32>,
    },

    /// Fixed-size struct allocated on the heap.
    Struct {
        /// Total payload size in bytes (excluding the object header).
        size: u32,
        /// Byte offsets within the payload that hold owned heap pointers.
        /// Move forbids references inside structs, so these are always
        /// 8-byte pointers to other heap objects (vectors, structs, etc.).
        pointer_offsets: Vec<u32>,
    },

    /// Enum (tagged union) allocated on the heap.
    /// Data-region layout: [tag: u64(8)] [fields padded to max variant size]
    Enum {
        /// Total payload size in bytes (tag + max variant fields, excluding header).
        size: u32,
        /// Per-variant pointer layouts. `variant_pointer_offsets[tag]` gives
        /// byte offsets (relative to `ENUM_DATA_OFFSET`) that hold heap
        /// pointers for that variant.
        variant_pointer_offsets: Vec<Vec<u32>>,
    },

    /// `ClosureCapturedData` (Materialized) object.
    ///
    /// Data-region layout: `[tag: u8 @ 0] [pad(3)] [values_size: u32 @ 4] [values @ 8]`.
    /// `pointer_offsets` are relative to the values region (i.e., excluding
    /// the 8-byte prefix), so an offset of `0` names the first byte of the
    /// first captured value. The prefix is added internally by the GC.
    /// Pointer-free captures carry no `CapturedData` descriptor: they share
    /// `Trivial`.
    CapturedData {
        /// Byte offsets within the values region that hold heap pointers.
        pointer_offsets: Vec<u32>,
    },
}

impl ObjectDescriptor {
    /// Construct the reserved [`Trivial`](ObjectDescriptorInner::Trivial)
    /// descriptor. Producers install one of these at [`TRIVIAL_DESCRIPTOR_ID`].
    pub const fn trivial() -> Self {
        Self(ObjectDescriptorInner::Trivial)
    }

    /// Construct the reserved [`Closure`](ObjectDescriptorInner::Closure)
    /// descriptor. Producers install one of these at [`CLOSURE_DESCRIPTOR_ID`].
    pub const fn closure() -> Self {
        Self(ObjectDescriptorInner::Closure)
    }

    /// Construct a [`Vector`](ObjectDescriptorInner::Vector) descriptor.
    ///
    /// Returns `Err` if `elem_size == 0`, `elem_pointer_offsets` is empty,
    /// or any offset in `elem_pointer_offsets` is not 8-byte aligned, runs
    /// past `elem_size`, or breaks strict ordering. A vector with no
    /// pointer offsets is pointer-free and uses the reserved `Trivial`
    /// descriptor instead.
    pub fn new_vector(elem_size: u32, elem_pointer_offsets: Vec<u32>) -> anyhow::Result<Self> {
        anyhow::ensure!(elem_size > 0, "Vector: elem_size must be > 0");
        anyhow::ensure!(
            !elem_pointer_offsets.is_empty(),
            "Vector: elem_pointer_offsets must be non-empty; pointer-free \
             vectors use the Trivial descriptor"
        );
        check_pointer_offsets(
            "Vector::elem_pointer_offsets",
            &elem_pointer_offsets,
            elem_size,
        )?;
        Ok(Self(ObjectDescriptorInner::Vector {
            elem_size,
            elem_pointer_offsets,
        }))
    }

    /// Construct a [`Struct`](ObjectDescriptorInner::Struct) descriptor.
    ///
    /// Returns `Err` if `size == 0` or any offset in `pointer_offsets`
    /// is not 8-byte aligned, runs past `size`, or breaks strict
    /// ordering.
    pub fn new_struct(size: u32, pointer_offsets: Vec<u32>) -> anyhow::Result<Self> {
        anyhow::ensure!(size > 0, "Struct: size must be > 0");
        check_pointer_offsets("Struct::pointer_offsets", &pointer_offsets, size)?;
        Ok(Self(ObjectDescriptorInner::Struct {
            size,
            pointer_offsets,
        }))
    }

    /// Construct an [`Enum`](ObjectDescriptorInner::Enum) descriptor.
    ///
    /// Returns `Err` if `size == 0`, `size < 8` (cannot hold the 8-byte
    /// tag), or any per-variant offset is not 8-byte aligned, runs past
    /// `size - 8` (the variant region after the tag), or breaks strict
    /// ordering within its variant.
    pub fn new_enum(size: u32, variant_pointer_offsets: Vec<Vec<u32>>) -> anyhow::Result<Self> {
        anyhow::ensure!(size > 0, "Enum: size must be > 0");
        // The 8-byte tag word lives at the start of the payload; variant
        // pointer offsets are relative to the data region after the tag,
        // so they must fit in `size - 8`.
        let variant_region = size.checked_sub(8).ok_or_else(|| {
            anyhow::anyhow!("Enum: size {} too small to hold the 8-byte tag", size)
        })?;
        for (variant, offsets) in variant_pointer_offsets.iter().enumerate() {
            check_pointer_offsets_indexed(
                "Enum::variant_pointer_offsets",
                variant,
                offsets,
                variant_region,
            )?;
        }
        Ok(Self(ObjectDescriptorInner::Enum {
            size,
            variant_pointer_offsets,
        }))
    }

    /// Construct a [`CapturedData`](ObjectDescriptorInner::CapturedData)
    /// descriptor. `values_size` is the byte width of the values region the
    /// `pointer_offsets` are validated against.
    ///
    /// Returns `Err` if `values_size == 0` or any offset in `pointer_offsets`
    /// is not 8-byte aligned, runs past `values_size`, or breaks strict
    /// ordering.
    pub fn new_captured_data(values_size: u32, pointer_offsets: Vec<u32>) -> anyhow::Result<Self> {
        anyhow::ensure!(values_size > 0, "CapturedData: values_size must be > 0");
        check_pointer_offsets(
            "CapturedData::pointer_offsets",
            &pointer_offsets,
            values_size,
        )?;
        Ok(Self(ObjectDescriptorInner::CapturedData {
            pointer_offsets,
        }))
    }

    /// Access the inner variant for pattern matching by the GC and
    /// verifier.
    pub fn inner(&self) -> &ObjectDescriptorInner {
        &self.0
    }
}

/// Reserved descriptor slot for [`ObjectDescriptorInner::Trivial`].
pub const TRIVIAL_DESCRIPTOR_ID: DescriptorId = DescriptorId(0);

/// Reserved descriptor slot for [`ObjectDescriptorInner::Closure`]. The
/// `PackClosure` micro-op uses this implicitly — it does not carry a
/// closure descriptor id of its own.
pub const CLOSURE_DESCRIPTOR_ID: DescriptorId = DescriptorId(1);

/// Number of reserved descriptors that every provider exposes (currently
/// `Trivial` and `Closure`). User descriptors are assigned ids starting
/// at this value.
pub const RESERVED_DESCRIPTOR_COUNT: u32 = 2;

// ---------------------------------------------------------------------------
// DescriptorProvider
// ---------------------------------------------------------------------------

/// Per-id lookup of object descriptors.
///
/// # Invariant
///
/// For every [`DescriptorId`] referenced by a function the interpreter
/// could execute next, [`Self::descriptor`] must return `Some`.
/// The verifier checks this invariant up-front for the entry function.
pub trait DescriptorProvider {
    /// Returns the descriptor for `id`, or `None` if `id` is not a
    /// descriptor known to this provider.
    fn descriptor(&self, id: DescriptorId) -> Option<&ObjectDescriptor>;
}

// ---------------------------------------------------------------------------
// ObjectDescriptorTable — a simple in-memory provider for tests/benches
// ---------------------------------------------------------------------------

/// In-memory table of [`ObjectDescriptor`] entries, indexed by
/// [`DescriptorId`]. Suitable for tests, benches, and small standalone
/// programs that need a `DescriptorProvider` without going through the
/// global context.
///
/// The table starts with the two reserved entries (`Trivial` at id `0`,
/// `Closure` at id `1`); user descriptors are appended via [`Self::push`].
/// Each user descriptor is validated by its constructor before reaching
/// the table, so any `&ObjectDescriptorTable` is structurally well-formed
/// by construction.
#[derive(Debug)]
pub struct ObjectDescriptorTable {
    descriptors: Vec<ObjectDescriptor>,
}

// `len_without_is_empty`: the table always has the two reserved entries,
// so `is_empty()` would be a tautological `false` — not worth providing.
#[allow(clippy::len_without_is_empty)]
impl ObjectDescriptorTable {
    /// Fresh table containing only the two reserved entries.
    pub fn new() -> Self {
        Self {
            descriptors: vec![ObjectDescriptor::trivial(), ObjectDescriptor::closure()],
        }
    }

    /// Append a user descriptor and return its assigned [`DescriptorId`].
    pub fn push(&mut self, desc: ObjectDescriptor) -> DescriptorId {
        let id = DescriptorId(
            u32::try_from(self.descriptors.len())
                .expect("descriptor table length exceeds u32::MAX"),
        );
        self.descriptors.push(desc);
        id
    }

    /// Number of entries (always at least [`RESERVED_DESCRIPTOR_COUNT`]).
    pub fn len(&self) -> usize {
        self.descriptors.len()
    }
}

impl Default for ObjectDescriptorTable {
    fn default() -> Self {
        Self::new()
    }
}

impl DescriptorProvider for ObjectDescriptorTable {
    fn descriptor(&self, id: DescriptorId) -> Option<&ObjectDescriptor> {
        self.descriptors.get(id.as_usize())
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Variant of [`check_pointer_offsets`] that defers the label allocation
/// to the error path. Hot construction loops (e.g. enums with many
/// variants) avoid the per-iteration `format!` on the success path.
fn check_pointer_offsets_indexed(
    label_prefix: &str,
    index: usize,
    offsets: &[u32],
    region_size: u32,
) -> anyhow::Result<()> {
    check_pointer_offsets_with_label(
        || format!("{}[{}]", label_prefix, index),
        offsets,
        region_size,
    )
}

/// Check that each offset in `offsets` names an 8-byte pointer in-bounds of
/// a region of `region_size` bytes (`off + 8 <= region_size`), is 8-byte
/// aligned, and that the list is strictly sorted (non-overlap follows).
fn check_pointer_offsets(label: &str, offsets: &[u32], region_size: u32) -> anyhow::Result<()> {
    check_pointer_offsets_with_label(|| label.to_string(), offsets, region_size)
}

fn check_pointer_offsets_with_label(
    label: impl Fn() -> String,
    offsets: &[u32],
    region_size: u32,
) -> anyhow::Result<()> {
    for &off in offsets {
        anyhow::ensure!(
            off % 8 == 0,
            "{}: offset {} is not 8-byte aligned",
            label(),
            off
        );
        let end = off
            .checked_add(8)
            .ok_or_else(|| anyhow::anyhow!("{}: offset {} + 8 overflows", label(), off))?;
        anyhow::ensure!(
            end <= region_size,
            "{}: pointer at offset {} (end {}) exceeds region size {}",
            label(),
            off,
            end,
            region_size
        );
    }
    for w in offsets.windows(2) {
        anyhow::ensure!(
            w[0] < w[1],
            "{}: offsets not strictly sorted ({} >= {})",
            label(),
            w[0],
            w[1]
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_has_reserved_entries() {
        let t = ObjectDescriptorTable::new();
        assert_eq!(t.len(), 2);
        assert!(matches!(
            t.descriptor(TRIVIAL_DESCRIPTOR_ID).map(|d| d.inner()),
            Some(ObjectDescriptorInner::Trivial)
        ));
        assert!(matches!(
            t.descriptor(CLOSURE_DESCRIPTOR_ID).map(|d| d.inner()),
            Some(ObjectDescriptorInner::Closure)
        ));
    }

    #[test]
    fn push_returns_increasing_ids() {
        let mut t = ObjectDescriptorTable::new();
        let a = t.push(ObjectDescriptor::new_vector(8, vec![0]).unwrap());
        let b = t.push(ObjectDescriptor::new_struct(16, vec![]).unwrap());
        assert_eq!(a, DescriptorId(2));
        assert_eq!(b, DescriptorId(3));
        assert_eq!(t.len(), 4);
    }

    fn err_msg<T>(r: anyhow::Result<T>) -> String {
        r.err().expect("expected Err").to_string()
    }

    // ----- Zero size -----

    #[test]
    fn vector_zero_elem_size_errors() {
        assert!(err_msg(ObjectDescriptor::new_vector(0, vec![])).contains("elem_size"));
    }

    #[test]
    fn struct_zero_size_errors() {
        assert!(err_msg(ObjectDescriptor::new_struct(0, vec![])).contains("Struct: size"));
    }

    #[test]
    fn enum_zero_size_errors() {
        assert!(err_msg(ObjectDescriptor::new_enum(0, vec![vec![]])).contains("Enum: size"));
    }

    #[test]
    fn captured_data_zero_size_errors() {
        assert!(err_msg(ObjectDescriptor::new_captured_data(0, vec![]))
            .contains("CapturedData: values_size"));
    }

    // ----- Pointer offsets -----

    #[test]
    fn struct_pointer_out_of_bounds_errors() {
        // 16 + 8 = 24 > 16
        assert!(err_msg(ObjectDescriptor::new_struct(16, vec![16])).contains("exceeds region size"));
    }

    #[test]
    fn struct_pointer_partially_out_of_bounds_errors() {
        // size 20 (allowed; runtime rounds up). Pointer at aligned offset
        // 16 starts in-bounds but `16 + 8 = 24 > 20`.
        assert!(err_msg(ObjectDescriptor::new_struct(20, vec![16])).contains("exceeds region size"));
    }

    #[test]
    fn misaligned_pointer_offset_errors() {
        assert!(err_msg(ObjectDescriptor::new_struct(24, vec![3])).contains("8-byte aligned"));
    }

    #[test]
    fn unsorted_pointer_offsets_errors() {
        assert!(
            err_msg(ObjectDescriptor::new_struct(32, vec![16, 8])).contains("not strictly sorted")
        );
    }

    #[test]
    fn vector_pointer_out_of_bounds_errors() {
        // 8 + 8 = 16 > 8
        assert!(err_msg(ObjectDescriptor::new_vector(8, vec![8])).contains("exceeds region size"));
    }

    #[test]
    fn vector_empty_pointer_offsets_errors() {
        // Pointer-free vectors are non-canonical as a Vector descriptor; they
        // use the reserved Trivial descriptor instead.
        assert!(err_msg(ObjectDescriptor::new_vector(8, vec![])).contains("non-empty"));
    }

    // ----- Enum tag accounting -----

    #[test]
    fn enum_size_smaller_than_tag_errors() {
        assert!(err_msg(ObjectDescriptor::new_enum(4, vec![vec![]])).contains("8-byte tag"));
    }

    #[test]
    fn enum_variant_pointer_out_of_bounds_errors() {
        // size 16 = tag(8) + variant region(8). Pointer at variant offset 8
        // means 8 + 8 = 16 > 8 (variant region).
        assert!(
            err_msg(ObjectDescriptor::new_enum(16, vec![vec![], vec![8]]))
                .contains("exceeds region size")
        );
    }

    #[test]
    fn captured_data_pointer_out_of_bounds_errors() {
        // 8 + 8 = 16 > 8
        assert!(err_msg(ObjectDescriptor::new_captured_data(8, vec![8]))
            .contains("exceeds region size"));
    }
}
