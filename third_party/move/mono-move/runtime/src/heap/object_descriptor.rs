// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Object descriptors and the program-wide [`ObjectDescriptorTable`].
//!
//! An [`ObjectDescriptor`] tells the GC how to trace internal pointers
//! within a single heap object. The descriptor table is a program-wide
//! `Vec<ObjectDescriptor>` indexed by [`DescriptorId`], with two reserved
//! entries (`Trivial` at 0, `Closure` at 1) that are populated implicitly
//! by [`ObjectDescriptorTable::new`].

use mono_move_core::DescriptorId;

// ---------------------------------------------------------------------------
// Object descriptors (for GC tracing)
// ---------------------------------------------------------------------------

/// Describes the reference layout of a heap object so the GC knows how to
/// trace internal pointers. Only one level of indirection is described;
/// pointed-to objects are self-describing via their own headers.
///
/// `ObjectDescriptor` is opaque to external callers — the wrapped
/// [`ObjectDescriptorInner`] enum is crate-private. External code creates
/// descriptors only through the validating constructors
/// ([`Self::new_vector`], [`Self::new_struct`], [`Self::new_enum`],
/// [`Self::new_captured_data`]) which return `anyhow::Result`; `Trivial`
/// and `Closure` are placed by [`ObjectDescriptorTable::new`] at their
/// reserved indices and cannot be constructed externally. Any
/// `&ObjectDescriptor` flowing through the runtime is therefore
/// self-sound by construction (nonzero size, in-bounds 8-byte-aligned
/// strictly-sorted pointer offsets).
#[derive(Debug)]
pub struct ObjectDescriptor(ObjectDescriptorInner);

/// Crate-private representation of an [`ObjectDescriptor`]. The runtime's
/// GC and verifier match on this directly via [`ObjectDescriptor::inner`].
#[derive(Debug)]
pub(crate) enum ObjectDescriptorInner {
    /// No internal heap references. GC copies the blob and moves on.
    Trivial,

    /// Closure object — fixed runtime layout shared by every closure.
    ///
    /// Payload layout (`size = CLOSURE_OBJECT_SIZE - OBJECT_HEADER_SIZE = 32`):
    /// `[func_ref(16)] [mask(8)] [captured_data_ptr(8)]`. The single heap
    /// pointer is `captured_data_ptr` at payload offset
    /// `CLOSURE_CAPTURED_DATA_PTR_OFFSET - OBJECT_HEADER_SIZE = 24`. Both
    /// the size and the pointer offset are constants of the runtime
    /// layout — there is nothing per-instance to store here.
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
    /// Layout: [header(8)] [tag: u64(8)] [fields padded to max variant size]
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
    /// Object layout: `[header(8)] [tag(1) + padding(7)] [values...]`.
    /// `size` and `pointer_offsets` are interpreted relative to the
    /// values region (i.e., excluding both the header and the
    /// tag+padding prefix), so an offset of `0` names the first byte of
    /// the first captured value. The 8-byte tag prefix is added
    /// internally by the GC.
    CapturedData {
        /// Byte size of the values region (sum of captured value sizes).
        size: u32,
        /// Byte offsets within the values region that hold heap pointers.
        pointer_offsets: Vec<u32>,
    },
}

impl ObjectDescriptor {
    /// Construct a [`Vector`](ObjectDescriptorInner::Vector) descriptor.
    ///
    /// Returns `Err` if `elem_size == 0` or any offset in
    /// `elem_pointer_offsets` is not 8-byte aligned, runs past
    /// `elem_size`, or breaks strict ordering.
    pub fn new_vector(elem_size: u32, elem_pointer_offsets: Vec<u32>) -> anyhow::Result<Self> {
        anyhow::ensure!(elem_size > 0, "Vector: elem_size must be > 0");
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
            let label = format!("Enum::variant_pointer_offsets[{}]", variant);
            check_pointer_offsets(&label, offsets, variant_region)?;
        }
        Ok(Self(ObjectDescriptorInner::Enum {
            size,
            variant_pointer_offsets,
        }))
    }

    /// Construct a [`CapturedData`](ObjectDescriptorInner::CapturedData)
    /// descriptor.
    ///
    /// Returns `Err` if `size == 0` or any offset in `pointer_offsets`
    /// is not 8-byte aligned, runs past `size`, or breaks strict
    /// ordering.
    pub fn new_captured_data(size: u32, pointer_offsets: Vec<u32>) -> anyhow::Result<Self> {
        anyhow::ensure!(size > 0, "CapturedData: size must be > 0");
        check_pointer_offsets("CapturedData::pointer_offsets", &pointer_offsets, size)?;
        Ok(Self(ObjectDescriptorInner::CapturedData {
            size,
            pointer_offsets,
        }))
    }

    /// Crate-internal access to the inner enum for pattern matching by
    /// the GC and verifier.
    pub(crate) fn inner(&self) -> &ObjectDescriptorInner {
        &self.0
    }
}

/// Reserved descriptor table index for [`ObjectDescriptorInner::Trivial`].
/// Every program's descriptor table has `Trivial` at this index.
pub const TRIVIAL_DESCRIPTOR_ID: DescriptorId = DescriptorId(0);

/// Reserved descriptor table index for [`ObjectDescriptorInner::Closure`].
/// Every program's descriptor table has `Closure` at this index. The
/// `PackClosure` micro-op uses this implicitly — it does not carry a
/// closure descriptor id of its own.
pub const CLOSURE_DESCRIPTOR_ID: DescriptorId = DescriptorId(1);

// ---------------------------------------------------------------------------
// Object descriptor table
// ---------------------------------------------------------------------------

/// Program-wide table of [`ObjectDescriptor`] entries, indexed by
/// [`DescriptorId`].
///
/// The table starts with two reserved entries:
///   - index `0` is [`ObjectDescriptorInner::Trivial`]
///   - index `1` is [`ObjectDescriptorInner::Closure`]
///
/// User descriptors are appended starting at index `2` via [`Self::push`].
/// `Trivial` and `Closure` aren't externally constructable; they only
/// live at their reserved indices.
///
/// Each user descriptor is validated by its constructor before reaching
/// the table (nonzero size, in-bounds 8-byte-aligned strictly-sorted
/// pointer offsets, `Enum` size large enough for the 8-byte tag). Any
/// `&ObjectDescriptorTable` is therefore structurally well-formed by
/// construction; no separate verification pass is needed.
///
/// Implements `Deref<Target = [ObjectDescriptor]>`, so it can be passed
/// anywhere a `&[ObjectDescriptor]` is expected.
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
            descriptors: vec![
                ObjectDescriptor(ObjectDescriptorInner::Trivial),
                ObjectDescriptor(ObjectDescriptorInner::Closure),
            ],
        }
    }

    /// Append a user descriptor and return its assigned [`DescriptorId`].
    /// Callers must capture the returned id and use it in micro-ops; the
    /// descriptor's index in the underlying storage is not part of the
    /// public API.
    ///
    /// `desc` is sound by construction (the public constructors on
    /// [`ObjectDescriptor`] validate inline), so this method does no
    /// further validation; it just appends.
    pub fn push(&mut self, desc: ObjectDescriptor) -> DescriptorId {
        let id = DescriptorId(
            u32::try_from(self.descriptors.len())
                .expect("descriptor table length exceeds u32::MAX"),
        );
        self.descriptors.push(desc);
        id
    }

    /// View the table as a slice indexed by [`DescriptorId`].
    pub fn as_slice(&self) -> &[ObjectDescriptor] {
        &self.descriptors
    }

    /// Number of entries (always at least 2).
    pub fn len(&self) -> usize {
        self.descriptors.len()
    }
}

impl Default for ObjectDescriptorTable {
    fn default() -> Self {
        Self::new()
    }
}

impl std::ops::Deref for ObjectDescriptorTable {
    type Target = [ObjectDescriptor];

    fn deref(&self) -> &[ObjectDescriptor] {
        self.as_slice()
    }
}

/// Check that each offset in `offsets` names an 8-byte pointer in-bounds of
/// a region of `region_size` bytes (`off + 8 <= region_size`), is 8-byte
/// aligned, and that the list is strictly sorted (non-overlap follows).
fn check_pointer_offsets(label: &str, offsets: &[u32], region_size: u32) -> anyhow::Result<()> {
    for &off in offsets {
        anyhow::ensure!(
            off % 8 == 0,
            "{}: offset {} is not 8-byte aligned",
            label,
            off
        );
        let end = off
            .checked_add(8)
            .ok_or_else(|| anyhow::anyhow!("{}: offset {} + 8 overflows", label, off))?;
        anyhow::ensure!(
            end <= region_size,
            "{}: pointer at offset {} (end {}) exceeds region size {}",
            label,
            off,
            end,
            region_size
        );
    }
    for w in offsets.windows(2) {
        anyhow::ensure!(
            w[0] < w[1],
            "{}: offsets not strictly sorted ({} >= {})",
            label,
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
        assert!(matches!(t[0].inner(), ObjectDescriptorInner::Trivial));
        assert!(matches!(t[1].inner(), ObjectDescriptorInner::Closure));
    }

    #[test]
    fn push_returns_increasing_ids() {
        let mut t = ObjectDescriptorTable::new();
        let a = t.push(ObjectDescriptor::new_vector(8, vec![]).unwrap());
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
        assert!(
            err_msg(ObjectDescriptor::new_captured_data(0, vec![])).contains("CapturedData: size")
        );
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

    // ----- Deref to slice -----

    #[test]
    fn deref_to_slice_works() {
        let t = ObjectDescriptorTable::new();
        let s: &[ObjectDescriptor] = &t;
        assert_eq!(s.len(), 2);
    }
}
