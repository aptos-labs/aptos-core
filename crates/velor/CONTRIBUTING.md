# Velor CLI Development Guide

This is a list of design decisions and guidelines for adding commands to the Velor CLI.

## Command Groups

Commands should be grouped into the existing categories. The current categories are:

- account
- config
- genesis
- governance
- key
- move
- multisig
- node
- stake
- update

All categories must have a doc comment that describes the command. It must also derive `Parser` and `Subcommand`. For
example:

```rust
/// Tool for interacting with accounts
///
/// This tool is used to create accounts, get information about the
/// account's resources, and transfer resources between accounts.
#[derive(Debug, Subcommand)]
pub enum AccountTool {
    Create(create::CreateAccount),
    CreateResourceAccount(create_resource_account::CreateResourceAccount),
    DeriveResourceAccountAddress(derive_resource_account::DeriveResourceAccount),
    FundWithFaucet(fund::FundWithFaucet),
    Balance(balance::Balance),
    List(list::ListAccount),
    LookupAddress(key_rotation::LookupAddress),
    RotateKey(key_rotation::RotateKey),
    Transfer(transfer::TransferCoins),
}
```

Then it must also be added to the top level command structure:

```rust
/// Command Line Interface (CLI) for developing and interacting with the Velor blockchain
#[derive(Parser)]
#[clap(name = "velor", author, version, propagate_version = true, styles = velor_cli_common::velor_cli_style())]
pub enum Tool {
    #[clap(subcommand)]
    Account(account::AccountTool),
    #[clap(subcommand)]
    Config(config::ConfigTool),
    #[clap(subcommand)]
    Genesis(genesis::GenesisTool),
    #[clap(subcommand)]
    Governance(governance::GovernanceTool),
    Info(InfoTool),
    Init(common::init::InitTool),
    #[clap(subcommand)]
    Key(op::key::KeyTool),
    #[clap(subcommand)]
    Move(move_tool::MoveTool),
    #[clap(subcommand)]
    Multisig(account::MultisigAccountTool),
    #[clap(subcommand)]
    Node(node::NodeTool),
    #[clap(subcommand)]
    Stake(stake::StakeTool),
    #[clap(subcommand)]
    Update(update::UpdateTool),
}
```

## Commands

A command is a single top level command for the CLI. The CLI command must complete it's action in the single command
execution.

### Command Names

```rust
/// Compiles a package and returns the associated ModuleIds
#[derive(Parser)]
pub struct CompilePackage {
    /// Save the package metadata in the package's build directory
    ///
    /// If set, package metadata should be generated and stored in the package's build directory.
    /// This metadata can be used to construct a transaction to publish a package.
    #[clap(long)]
    pub(crate) save_metadata: bool,

    #[clap(flatten)]
    pub(crate) included_artifacts_args: IncludedArtifactsArgs,
    #[clap(flatten)]
    pub(crate) move_options: MovePackageDir,
}
```

Command names should be simple, identifiable, and easy to use. For example, compilation is grouped in `move` and uses
the subcommand `compile`.

```bash
velor move compile
```

Once the new command is created, it should have `#[derive(Parser)]` added above. Additionally, it will need to be added
the higher level tool:

```rust
#[derive(Subcommand)]
pub enum MoveTool {
    #[clap(alias = "build")]
    Compile(CompilePackage),
    #[clap(alias = "build-script")]
    CompileScript(CompileScript),
    Init(Init),
    // ...
}

impl MoveTool {
    pub async fn execute(self) -> CliResult {
        match self {
            MoveTool::Compile(tool) => tool.execute_serialized().await,
            MoveTool::CompileScript(tool) => tool.execute_serialized().await,
            MoveTool::Init(tool) => tool.execute_serialized_success().await,
        }
    }
}
```

Note that, there are two types of commands here `execute_serialized()` and `execute_serialized_success()`, if the
command must be returning a value, then it should call `execute_serialized()`, which will convert the input type as JSON
to `stdout`.

Additionally, `alias` is allowed, but discouraged for new commands. This is mostly to provide either backwards
compatibility or reduce confusion for new users.

### Command flags

```rust
#[derive(Parser)]
pub struct CompilePackage {
    /// Save the package metadata in the package's build directory
    ///
    /// If set, package metadata should be generated and stored in the package's build directory.
    /// This metadata can be used to construct a transaction to publish a package.
    #[clap(long)]
    pub(crate) save_metadata: bool,

    // ...
}
```

Command inputs should always be documented for help to show up in the CLI. for example, below is the example for
`save_metadata`. They should be snake case, and will show up as a flag. Do not use `short` commands, as they can be
confused between different commands.

```bash
velor move compile --save-metadata
```

### Command flag groupings

```rust
/// Compiles a package and returns the associated ModuleIds
#[derive(Parser)]
pub struct CompilePackage {
    // ...
    #[clap(flatten)]
    pub(crate) included_artifacts_args: IncludedArtifactsArgs,
    #[clap(flatten)]
    pub(crate) move_options: MovePackageDir,
}
```

Command flags can be grouped into common structs to be used across multiple commands. These should be flattened by
adding the struct associated and using `#[clap(flatten)]` like above. These should not have a doc comment, and any doc
comments will not end up in the command. Instead, document the structs directly like so:

```rust
#[derive(Parser)]
pub struct IncludedArtifactsArgs {
    /// Artifacts to be generated when building the package
    ///
    /// Which artifacts to include in the package. This can be one of `none`, `sparse`, and
    /// `all`. `none` is the most compact form and does not allow to reconstruct a source
    /// package from chain; `sparse` is the minimal set of artifacts needed to reconstruct
    /// a source package; `all` includes all available artifacts. The choice of included
    /// artifacts heavily influences the size and therefore gas cost of publishing: `none`
    /// is the size of bytecode alone; `sparse` is roughly 2 times as much; and `all` 3-4
    /// as much.
    #[clap(long, default_value_t = IncludedArtifacts::Sparse)]
    pub(crate) included_artifacts: IncludedArtifacts,
}
```

### Command Implementation

```rust
#[async_trait]
impl CliCommand<Vec<String>> for CompilePackage {
    fn command_name(&self) -> &'static str {
        "CompilePackage"
    }

    async fn execute(self) -> CliTypedResult<Vec<String>> {
        let build_options = BuildOptions {
            install_dir: self.move_options.output_dir.clone(),
            ..self
                .included_artifacts_args
                .included_artifacts
                .build_options(
                    self.move_options.dev,
                    self.move_options.skip_fetch_latest_git_deps,
                    self.move_options.named_addresses(),
                    self.move_options.override_std.clone(),
                    self.move_options.bytecode_version,
                    self.move_options.compiler_version,
                    self.move_options.language_version,
                    self.move_options.skip_attribute_checks,
                    self.move_options.check_test_code,
                )
        };
        let pack = BuiltPackage::build(self.move_options.get_package_path()?, build_options)
            .map_err(|e| CliError::MoveCompilationError(format!("{:#}", e)))?;
        if self.save_metadata {
            pack.extract_metadata_and_save()?;
        }
        let ids = pack
            .modules()
            .map(|m| m.self_id().to_string())
            .collect::<Vec<_>>();
        // TODO: Also say how many scripts are compiled
        Ok(ids)
    }
}
```

Commands should implement the `CliCommand<T>` trait for the package. This allows it to be called upstream generically
and `T` will automatically be serialized to JSON for the output. This allows for typed testing in unit tests, while
still having output converted for the total CLI.

It's an anti-pattern to `panic`, please avoid panicking, and instead provide `CliError` or `CliError` conversion for the
current types.

All output from the CLI should use `eprintln!()`, rather than `println!()`.  `stdout` is reserved for the JSON output at
the end of the command, `stderr` is used for the rest of the output.
