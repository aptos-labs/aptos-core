# AptosVMv2: Session Continuation Design

## Executive Summary

AptosVMv2 introduces a fundamental shift from a **multi-session squashing model** to a **single-session continuation model** with copy-on-write semantics. This eliminates write set squashing between transaction stages, addressing both correctness and performance issues.

## Old Model: SessionExt with Write Set Squashing

### Architecture
- **Multi-Session**: Each transaction stage (prologue, execution, epilogue) used separate `SessionExt` instances
- **Finalization at Each Stage**: Between stages, sessions were finalized via `finish()` or `finish_with_squashed_change_set()`
- **Write Set Squashing**: Intermediate write sets were squashed and passed between stages
- **Linear Flow**: Session₁ → WriteSet₁ → Session₂ → WriteSet₂ → Session₃ → FinalWriteSet

### Problems Identified
1. **Bug Source**: Write set squashing logic was complex and error-prone
2. **Performance Limitation**: Constant serialization/deserialization between stages
3. **State Management**: No ability to rollback or checkpoint intermediate states

## New Model: AptosVMv2 with Session Continuation

### Architecture
- **Single Session**: One `Session<DataView, CodeLoader>` spans entire transaction execution
- **Versioned State**: Uses `VersionController` and `VersionedSlot<T>` for state management (types/src/vm/versioning.rs:13)
- **Copy-on-Write**: Values are copied only when modified using `clone_value()` (types/src/vm/versioning.rs:197)
- **Session Checkpointing**: `save_state_changes()` and `undo_state_changes()` provide rollback capability

### Key Components

#### Version Control System
```rust
pub struct VersionController {
    next_version: u32,
    saved_versions: SmallVec<[u32; 3]>,
    current_version: u32,
}
```

#### Versioned Data Storage
```rust
pub struct VersionedSlot<V: Copyable> {
    versions: SmallVec<[VersionedValue<V>; 3]>,
}
```

#### Session State Management
- **Save**: `session.data_cache.save()` + `session.extensions.for_each_mut(|e| e.save())`
- **Undo**: `session.data_cache.undo()` + `session.extensions.for_each_mut(|e| e.undo())`

### Execution Flow Comparison

#### Old Flow
```
Prologue Session → finish() → WriteSet₁
  ↓ (squash + new session)
Execution Session → finish() → WriteSet₂  
  ↓ (squash + new session)
Epilogue Session → finish() → FinalWriteSet
```

#### New Flow
```
Single Session:
  save_state() → Prologue → save_state() → Execution → save_state() → Epilogue
                    ↓              ↓                      ↓
               (checkpoint)   (checkpoint)           (checkpoint)
                    ↓              ↓                      ↓
            (can undo on error) (can undo on error) (can undo on error)
```

## Benefits of the New Model

### 1. **Correctness**
- Eliminates complex write set squashing logic that was bug-prone
- Atomic rollback capability prevents inconsistent intermediate states
- Single source of truth for transaction state

### 2. **Performance**
- **Zero Serialization**: No intermediate write set creation between stages
- **Copy-on-Write**: Only modified values are copied (types/src/vm/versioning.rs:196-199)
- **Memory Efficiency**: Versioned slots track minimal state changes

### 3. **Robustness**
- **Error Recovery**: Can rollback to any saved checkpoint
- **State Isolation**: Each stage can be undone without affecting others
- **Debugging**: Clear state progression with version tracking

## Implementation Details

### Data Cache Architecture
- `TransactionDataCache` with `VersionController` for state versioning (v2/data_cache.rs:115)
- Resource entries use `VersionedSlot<GlobalValue>` for copy-on-write semantics (v2/data_cache.rs:101)
- Materialization tracking through `VersionedSlot<MaterializedGlobalValue>` (v2/data_cache.rs:102)

### Session Continuation API
```rust
impl UserTransactionSession {
    pub fn save_state_changes(&mut self) // Create checkpoint
    pub fn undo_state_changes(&mut self)  // Rollback to checkpoint
}
```

This design represents a fundamental architectural improvement, moving from a **stateless stage-by-stage** approach to a **stateful continuation** model that maintains consistency while improving performance.