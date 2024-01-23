// Copyright (c) The BitsLab.MoveBit Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::multiproject::MultiProject;
use lsp_server::Connection;
use std::{collections::HashMap, path::PathBuf};

/// The context within which the language server is running.
pub struct Context {
    /// The connection with the language server's client.
    pub connection: Connection,
    pub projects: MultiProject,
    pub diag_version: FileDiags,
}

#[derive(Default)]
pub struct FileDiags {
    diags: HashMap<PathBuf, HashMap<url::Url, usize>>,
}

impl FileDiags {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn update(&mut self, mani: &PathBuf, fpath: &url::Url, diags: usize) {
        if let Some(x) = self.diags.get_mut(mani) {
            x.insert(fpath.clone(), diags);
        } else {
            let mut x: HashMap<url::Url, usize> = HashMap::new();
            x.insert(fpath.clone(), diags);
            self.diags.insert(mani.clone(), x);
        }
    }

    pub fn with_manifest(&self, mani: &PathBuf, mut call: impl FnMut(&HashMap<url::Url, usize>)) {
        let empty = Default::default();
        call(self.diags.get(mani).unwrap_or(&empty));
    }
}
