// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Loaded module — what the executable cache stores.
//!
//! [`LoadedModule`] wraps the polymorphic [`ModuleIR`] with the monomorphic
//! [`Executable`] and a [`MandatoryDependencies`] descriptor.

use mono_move_alloc::{GlobalArenaPtr, LeakedBoxPtr, VersionedLeakedBoxPtr};
use mono_move_core::{Executable, ExecutableId};
use specializer::ModuleIR;
use std::sync::Arc;

/// Stable slot pointer for a loaded module in the cache. May be empty if the
/// module has not yet been cached.
pub type LoadedModuleSlot = LeakedBoxPtr<VersionedLeakedBoxPtr<LoadedModule>>;

/// What a loaded module says about its mandatory dependencies, keyed by the
/// loading policy that built it. Covers every package member including self
/// for package loads, and is empty for lazy loads.
#[derive(Clone)]
pub struct MandatoryDependencies {
    inner: Option<Arc<[LoadedModuleSlot]>>,
}

impl MandatoryDependencies {
    /// Slots of the modules this module loaded together with.
    pub fn slots(&self) -> &[LoadedModuleSlot] {
        self.inner.as_ref().map(|r| r.as_ref()).unwrap_or(&[])
    }

    pub fn empty() -> MandatoryDependencies {
        MandatoryDependencies { inner: None }
    }

    pub fn package(package_slots: Vec<LoadedModuleSlot>) -> MandatoryDependencies {
        MandatoryDependencies {
            inner: Some(Arc::from(package_slots)),
        }
    }
}

/// A loaded module: polymorphic IR + the monomorphic executable view + the
/// dependency descriptor produced at load time.
pub struct LoadedModule {
    ir: ModuleIR,
    executable: Executable,
    mandatory_dependencies: MandatoryDependencies,
}

impl LoadedModule {
    pub fn new(
        ir: ModuleIR,
        executable: Executable,
        mandatory_dependencies: MandatoryDependencies,
    ) -> Box<Self> {
        Box::new(Self {
            ir,
            executable,
            mandatory_dependencies,
        })
    }

    /// Returns the polymorphic stackless IR.
    pub fn ir(&self) -> &ModuleIR {
        &self.ir
    }

    /// Returns the monomorphic executable view.
    pub fn executable(&self) -> &Executable {
        &self.executable
    }

    /// Returns the mandatory-dependency descriptor.
    pub fn mandatory_dependencies(&self) -> &MandatoryDependencies {
        &self.mandatory_dependencies
    }

    /// Convenience: the executable's ID. Same as `self.executable().id()`.
    pub fn id(&self) -> GlobalArenaPtr<ExecutableId> {
        self.executable.id()
    }

    /// Convenience: the executable's deterministic load cost.
    pub fn cost(&self) -> u64 {
        self.executable.cost()
    }
}
