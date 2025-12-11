# Build Binary Action

A TypeScript GitHub Action that builds Rust binaries using Nix for the Aptos Core project. This action intelligently discovers and builds all binaries produced by the Cargo workspace, making it maintenance-free and adaptable to workspace changes.

## Features

- ✅ **Smart Binary Discovery** - Automatically discovers binaries built in the target folder
- ✅ **Zero Maintenance** - No hardcoded binary lists; adapts to workspace changes
- ✅ **Efficient Building** - Builds all default-members in a single Cargo invocation
- ✅ **Flexible** - Supports building additional non-default-member binaries
- ✅ **Type-Safe** - Written in TypeScript with full type safety
- ✅ **Production Ready** - Includes comprehensive error handling and logging

## How It Works

### Build Process

1. **Build default-members**: Runs `cargo build --release` to build all packages in the workspace's `default-members` list
2. **Build additional binaries**: Optionally builds specified non-default-member binaries with `cargo build --release -p <binary>`
3. **Discover binaries**: Scans the target folder for executable files (actual binaries, not libraries)
4. **Verify**: Ensures all requested additional binaries were built
5. **Upload**: Uploads all discovered binaries as a single artifact

### Smart Discovery

Instead of hardcoding which binaries to expect, the action:
- Scans the target folder for executable files
- Filters out libraries (`.rlib`, `.d`, etc.)
- Only includes actual binary executables
- Adapts automatically when binaries are added/removed from the workspace

This approach works because:
- Not all `default-members` produce binaries (some are libraries like `aptos-move/framework`)
- Binary names may differ from package directory names
- The workspace configuration may change over time

## Usage

### Basic Usage

Build all default-members from Cargo.toml:

```yaml
- name: Build all binaries
  uses: ./.github/actions/build-binary
  with:
    profile: release
```

### With Additional Binaries

Build default-members plus additional binaries:

```yaml
- name: Build all binaries
  uses: ./.github/actions/build-binary
  with:
    binaries: l1-migration
    profile: release
```

### Multiple Additional Binaries

```yaml
- name: Build all binaries
  uses: ./.github/actions/build-binary
  with:
    binaries: l1-migration,custom-tool,another-binary
    profile: release
```

### Skip Default Members

Build only specific binaries:

```yaml
- name: Build specific binaries
  uses: ./.github/actions/build-binary
  with:
    defaults: false
    binaries: l1-migration,aptos-node
    profile: release
```

### Complete Workflow Example

```yaml
jobs:
  build:
    runs-on: k8s-movement-labs
    steps:
      - uses: actions/checkout@v4
      
      # Install system dependencies
      - name: Install dependencies
        run: sudo apt-get update && sudo apt-get install -y xz-utils
      
      # Install Nix
      - name: Install Nix
        uses: cachix/install-nix-action@v27
        with:
          github_access_token: ${{ secrets.GITHUB_TOKEN }}
          nix_path: nixpkgs=channel:nixos-unstable
      
      # Build all binaries
      - name: Build binaries
        uses: ./.github/actions/build-binary
        with:
          binaries: l1-migration
          profile: release
```

## Inputs

| Input | Required | Default | Description |
|-------|----------|---------|-------------|
| `defaults` | No | `true` | Whether to build all default-members from Cargo.toml |
| `binaries` | No | `""` | Comma-separated list of additional binaries to build |
| `profile` | Yes | - | Cargo build profile (`release`, `dev`, or custom profile name) |

## Outputs

| Output | Description |
|--------|-------------|
| `artifact_name` | Name of the uploaded artifact (e.g., `all-binaries-abc1234`) |
| `binaries_built` | JSON array of binary names that were built |

## Prerequisites

The action expects:
1. **Nix to be installed** - Use `cachix/install-nix-action@v27` before this action
2. **System dependencies** - Install `xz-utils` before this action
3. **Cargo workspace** - A valid `Cargo.toml` with workspace configuration

## Example Output

When the action runs, you'll see:

```
🔨 Build Configuration:
  Profile: release
  Build defaults: true
  Additional binaries: l1-migration

📦 Building all default-member binaries...
   (This builds all packages in default-members that produce binaries)
✅ Default-members build complete

📦 Building additional binaries...
  Building: l1-migration
✅ Additional binaries build complete

📁 Target folder: target/release

📋 Discovering built binaries in target folder...

  ✅ aptos (2.89 GB)
  ✅ aptos-faucet-service (1.03 GB)
  ✅ aptos-node (2.04 GB)
  ✅ l1-migration (972.35 MB)

📊 Total binaries found: 4

📤 Uploading 4 binaries as artifact: all-binaries-abc1234
✅ Artifact uploaded: all-binaries-abc1234
   ID: 12345
   Size: 6442450944 bytes
```

## Artifact Usage

Download the artifact in subsequent jobs:

```yaml
- name: Download binaries
  uses: actions/download-artifact@v4
  with:
    name: all-binaries-${{ needs.setup.outputs.short_sha }}
    path: target/release
```

All binaries will be available in the specified path.

## Development

### Project Structure

```
.github/actions/build-binary/
├── action.yml          # Action definition
├── package.json        # Dependencies
├── tsconfig.json       # TypeScript config
├── src/
│   └── main.ts        # Main action logic
├── dist/              # Compiled output (committed)
│   └── index.js       # Bundled action
└── README.md          # This file
```

### Building

```bash
cd .github/actions/build-binary

# Install dependencies
npm install

# Build the action
npm run build

# The compiled output will be in dist/
```

### Making Changes

1. Edit `src/main.ts`
2. Run `npm run build`
3. Commit both `src/` and `dist/` changes
4. Test in a workflow

**Important**: Always commit the `dist/` folder! GitHub Actions runs the compiled code from `dist/index.js`, not the TypeScript source.

## Technical Details

### Dependencies

- `@actions/core` - GitHub Actions core library
- `@actions/exec` - Command execution
- `@actions/artifact` - Artifact upload/download
- `@iarna/toml` - TOML parser (for future enhancements)
- TypeScript & build tools

### Binary Discovery Algorithm

```typescript
1. List all files in target/{profile}/
2. For each file:
   - Check if it's a regular file
   - Check if it has executable permissions
   - Skip files with extensions (libraries)
3. Return list of discovered binaries
```

### Why Not Parse Cargo.toml?

While we could parse `Cargo.toml` to get the list of default-members, this approach has limitations:
- Not all default-members produce binaries (some are libraries)
- Binary names may differ from package names
- Requires maintaining mapping logic

Instead, we let Cargo build what it needs to build, then discover what was actually produced. This is more robust and requires zero maintenance.

## Troubleshooting

### No binaries found

**Problem**: Action reports "No binaries were built!"

**Solutions**:
- Ensure Nix is installed before this action
- Check that `cargo build` completes successfully
- Verify the profile name is correct (`release`, `dev`, etc.)

### Expected binary not found

**Problem**: Action reports "Expected binary not found: xyz"

**Solutions**:
- Verify the binary name in the `binaries` input matches the actual binary name
- Check if the binary is in `default-members` (if so, don't specify it in `binaries`)
- Ensure the package builds successfully

### Build fails

**Problem**: Cargo build fails

**Solutions**:
- Check Nix installation
- Verify system dependencies are installed
- Review Cargo build errors in the logs
- Ensure the workspace is in a clean state

## License

Apache-2.0 (same as Aptos Core)

## Contributing

This action is part of the Aptos Core project. See the main [CONTRIBUTING.md](../../../CONTRIBUTING.md) for contribution guidelines.