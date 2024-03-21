// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

/// Container for experiments in the compiler. Those can be activated
/// via the `--experiment=<name>` flag. One can also use the env var
/// `MVC_EXP=<comma list>` to activate those flags.
///
/// Moreover, one can activate an experiment in a test source by using a
/// comment as such in a unit test:
/// ```text
///   // experiment: <name>
/// ```
/// This can be repeated an arbitrary number of times. The test will then be run for
/// the default configuration and for each of the named experiments separately (if it is a
/// baseline test, a different baseline file will be generated each time).
///
/// Each new experiment should have a description and explicit note about its retention.
///
/// - Permanent: the experiment is available indefinitely
/// - Temporary: the experiment is intended to be removed after some time. Please document
///   the condition under which it can be removed.
pub struct Experiment();

impl Experiment {
    /// This flag enables running various AST-based aggressive optimizations, such as
    /// code eliminations, which are turned off by default.
    /// Retention: temporary. To be removed when various optimization pipeline combos
    /// can be controlled via user-accessible configs.
    pub const AST_AGGRESSIVE_OPTIMIZE: &'static str = "ast-aggressive-optimize";
    /// A flag which allows to turn off safety checks, or suppress any error messages
    /// they produce.
    /// Retention: permanent.
    pub const NO_SAFETY: &'static str = "no-safety";
    /// The compiler runs a default pipeline of stackless bytecode optimizations.
    /// This flag allows to turn off these optimizations.
    /// Retention: temporary. To be removed when various optimization pipeline combos
    /// can be controlled via user-accessible configs.
    pub const NO_SBC_OPTIMIZE: &'static str = "no-sbc-optimize";
    /// A flag which allows to turn on the critical edge splitting pass.
    /// Retention: temporary. This should be removed after the pass can be tested.
    pub const SPLIT_CRITICAL_EDGES: &'static str = "split-critical-edges";
}
