# Stack Memory Model & Calling Convention

The VM's interpreter loop manages a single flat linear memory buffer as its unified call stack. Both frame data (parameters, locals) and frame metadata (return address, saved frame pointer, saved function pointer) reside in this buffer. The frame pointer (`fp`) points to the beginning of the current callee's frame.

```
              caller frame                           callee frame
  ┌──────────────────────────────────┐   ┌──────────────────────────────────┐
  │                        │ saved  ││   │                                  │
  │  caller locals         │  pc    ││   │ params │  callee locals  │ ...   │
  │                        │  fp    ││   │                                  │
  │                        │func_ptr││   │                                  │
  └──────────────────────────────────┘   └──────────────────────────────────┘
                           ▲             ▲
                      metadata (24B)     fp
```

## Call Sequence

When the caller invokes a function:

1. The caller writes a 24-byte metadata section `(saved_pc, saved_fp, saved_func_ptr)` at the end of its own frame. This records the return address (program counter), the caller's frame pointer, and a pointer to the current function (`NonNull<Function>`).
2. The caller writes the callee's arguments into a contiguous region immediately following the metadata, at the beginning of the callee's frame (the callee's parameter region).
3. `fp` is set to the start of the callee's frame.

> **Resolved**: The metadata stores a raw function pointer (`NonNull<Function>`) rather than a function table index, eliminating the need for a function table lookup on return.

## Return Sequence

When the callee returns:

1. The callee stores all return values at the beginning of its frame, contiguously, potentially overwriting some of its own parameters or locals.
2. The interpreter reads the saved metadata at `fp - 24` to restore the caller's `pc`, `fp` and `func_ptr`.

## Local Access

Instructions access locals via `fp + offset`, where offsets are computed at compile time (during monomorphization). This avoids runtime index lookups and keeps the common case — reading or writing a local — to a single base-plus-offset memory access.

## Comparison with x86-64

This calling convention resembles x86-64, with one difference: x86-64 uses a mirrored layout (locals at `rbp - offset`, metadata at `rbp + offset`) because the stack grows downward. MonoMove's stack grows upward, so both metadata and locals are at positive offsets from the frame boundary.

## Unified Stack vs. Separate Call Stack

The unified stack approach was chosen for the runtime. The trade-offs considered:

|  | Unified Stack | Separate Call Stack |
| --- | --- | --- |
| **Memory locality** | All frame data in one contiguous buffer — better cache behavior | Metadata and data in separate allocations — worse locality |
| **Return overhead** | 3 loads from a known offset — no lookup needed | Requires a `Vec<Frame>` pop on every return |
| **Bookkeeping** | No auxiliary data structures | Additional `Vec` management |
| **Security** | Mixing control flow metadata with data means a memory corruption bug could lead to control flow hijacking | Clear separation — data corruption cannot hijack control flow |
| **Measured performance** | ~1.28x faster on a recursive Fibonacci (call-heavy) benchmark | Baseline |

### Alternative Approach (not chosen): Separate Call Stack

The alternative is to store frame metadata `(pc, fp, func_ptr)` in a separate `Vec<Frame>`, while keeping frame data (parameters, locals) in the flat linear buffer.

- This cleanly separates control flow metadata from data, which provides a stronger security property: data corruption (e.g., from a VM bug or a crafted value) cannot directly hijack the control flow.
- The cost is additional overhead from maintaining the separate structure (a vector push/pop per call/return) and worse cache locality.

## Optional Optimization: Per-Function Calling Convention Customization

Rather than requiring parameters and return values to occupy the beginning of the frame in a fixed contiguous layout, each function could declare custom offsets for its parameters and return values, determined at compile time. This could eliminate some unnecessary moves.

However, the benefits may not justify the complexity:

- Small, simple functions benefit more from being inlined entirely.
- Complex functions are unlikely to see meaningful gains from saving a few moves relative to the cost of the function body itself.

## GC Root Discovery: `frame_layout` and `safe_point_layouts`

The garbage collector needs to find all live heap pointers on the call stack. The runtime uses a two-level scheme:

1. **`frame_layout`** (`FrameLayoutInfo`) — a per-function list of frame byte-offsets that *always* hold heap pointers, regardless of the current PC. The GC scans these offsets in every live frame.

2. **`safe_point_layouts`** (`[SafePointEntry]`) — per-safe-point lists of *additional* frame offsets that hold heap pointers at specific code offsets. The GC looks up the entry matching the frame's current PC (via binary search) and scans those offsets too.

At any given safe point, the full set of GC roots on the frame is the union of `frame_layout.heap_ptr_offsets` and the matching safe-point entry's `heap_ptr_offsets` (if any).

**Safe points** are the only instructions where GC can trigger:

- **Allocating instructions** (`HeapNew`, `VecPushBack`, `ForceGC`): GC runs during the instruction, so the safe point is at that instruction's own PC.
- **Call return sites**: when a callee triggers GC, the caller's saved PC is `call_pc + 1`. The safe point for a caller frame is the instruction *after* the call — at that point, the shared arg/return region holds return values, not arguments.

When `zero_frame` is true, pointer slots are zeroed on entry so the GC always sees either a valid heap pointer or null — never stale data. `FrameLayoutInfo` is designed to be extended with additional per-slot type or layout information in the future (e.g., slot type tags for debugging or stronger runtime verification).

The two-level scheme exists because the calling convention requires parameters and return values to share the same space at the beginning of the frame. A slot might hold a heap pointer as a parameter on entry, but later be overwritten with a scalar return value (or vice versa). Similarly, callee arg slots may hold pointers for one callee but scalars for another. A single per-function list cannot describe both states. The `safe_point_layouts` mechanism handles this: slots that change type across call boundaries are listed in the appropriate safe-point entries rather than in `frame_layout`. The specializer emits safe-point entries only at PCs where the pointer set differs from the base `frame_layout` — functions with no type-changing slots leave `safe_point_layouts` empty.

If we move toward **strongly-typed slots** (where each frame offset has a fixed type for the duration of the frame), the safe-point mechanism provides the necessary per-PC type information to support this.

This approach was chosen because it is well understood, stable, and known to work. We may revisit this and explore alternatives in the future — for example, removing the per-function layout entirely in favor of per-PC-only layouts, or moving some of the work into the runtime (e.g., having the runtime reconstruct per-PC layout info rather than requiring the specializer to emit it). See `docs/heap_and_gc.md` for the full GC design space (Approaches A–D) and the rationale.

## Security Considerations

The stack is a high-value attack surface because it holds both execution state and user-controlled data. The following concerns are specific to the stack memory model and calling convention (see the main design doc Section 7 for general VM security invariants).

- **Stack overflow.** Unbounded or deeply recursive Move calls can exhaust the stack buffer. The VM must enforce a stack depth or size limit and abort the transaction cleanly when it is exceeded — never allow a write past the end of the allocated buffer.
- **Control flow hijacking (unified stack only).** When frame metadata and frame data coexist in the same buffer, a bug that corrupts stack memory (e.g., an out-of-bounds write to a local) could overwrite the saved `pc` or `fp`, redirecting execution to an arbitrary location. This is the primary security argument for the separate call stack alternative. If the unified stack is chosen, defense-in-depth measures should be considered, such as frame canaries or integrity checks on metadata at return time.
- **Out-of-bounds local access.** Since locals are accessed via `fp + offset` with compile-time offsets, an incorrect offset (e.g., from a monomorphization bug or a malformed instruction) could read or write outside the current frame. Bounds checking against the frame size at access time would catch this, at some performance cost.
- **Uninitialized memory.** When a new frame is allocated, the memory region may contain leftover data from a previous frame. The runtime addresses this via the `zero_frame` flag on `Function`: when true, the region beyond parameters (`param_sizes_sum..extended_frame_size`) is zeroed at frame creation, ensuring pointer slots start as null for GC safety. Functions with no heap pointer slots can set `zero_frame = false` to skip the memset.
- **Return value overwrites.** The calling convention allows the callee to overwrite its own parameters and locals when writing return values. This is safe by construction if offsets are correct, but an off-by-one in the return value layout could corrupt the caller's metadata (in the unified stack) or adjacent data. The compiler must guarantee that return value writes stay within the callee's frame.
