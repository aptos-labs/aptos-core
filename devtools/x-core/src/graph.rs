// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::{Result, SystemError, WorkspaceSubsets, XCoreContext};
use guppy::{graph::PackageGraph, MetadataCommand};
use ouroboros::self_referencing;

#[self_referencing]
pub(crate) struct PackageGraphPlus {
    g: Box<PackageGraph>,
    #[borrows(g)]
    #[covariant]
    subsets: WorkspaceSubsets<'this>,
}

impl PackageGraphPlus {
    pub(crate) fn create(ctx: &XCoreContext) -> Result<Self> {
        let mut cmd = MetadataCommand::new();
        // Run cargo metadata from the root of the workspace.
        let project_root = ctx.project_root();
        cmd.current_dir(project_root);

        Self::try_new(
            Box::new(
                cmd.build_graph()
                    .map_err(|err| SystemError::guppy("building package graph", err))?,
            ),
            move |graph: &PackageGraph| {
                // Skip over the hakari package because it is not included in release or other builds.
                // (Can't use ctx.hakari_builder() because we're in the middle of constructing it.)
                let hakari_name = ctx
                    .config
                    .hakari
                    .builder
                    .hakari_package
                    .as_deref()
                    .expect("hakari package is specified");
                let hakari_package = graph
                    .workspace()
                    .member_by_name(hakari_name)
                    .expect("hakari package exists in workspace");

                WorkspaceSubsets::new(graph, project_root, &ctx.config().subsets, &hakari_package)
            },
        )
    }

    pub(crate) fn package_graph(&self) -> &PackageGraph {
        self.borrow_g()
    }

    pub(crate) fn subsets(&self) -> &WorkspaceSubsets {
        self.borrow_subsets()
    }
}
