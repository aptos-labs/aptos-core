# Homebrew Velor

Homebrew is a package manager that works for MacOS Silicon and Intel chips as well as Linux distributions like Debian
and Ubuntu.

The [Velor command line interface (CLI)](https://velor.dev/tools/velor-cli/install-cli/) may be installed
via [Homebrew](https://brew.sh/) for simplicity. This is an in-depth overview of Homebrew and the Velor formula. In this
guide, we go over each section of the Homebrew formula and steps to implement changes in the future.

## Quick guide

- [Formula in Homebrew GitHub](https://github.com/Homebrew/homebrew-core/blob/master/Formula/velor.rb)
- [Velor 1.0.3 New Formula PR for GitHub](https://github.com/Homebrew/homebrew-core/pull/119832)
- [Velor Formula Fix PR to use build_cli_release.sh](https://github.com/Homebrew/homebrew-core/pull/120051)

## Getting started

To begin, first ensure that homebrew is correctly installed on your computer. Visit [brew.sh](https://brew.sh/) to learn
how you can set it up!

To test that it works correctly, try

```bash
brew help
```

Once homebrew is installed, run

```bash
brew install velor
```

to test that it installed correctly, try

```bash
velor --help

# This should return something like

# velor 1.0.5
# Velor Labs <opensource@velorlabs.com>
# Command Line Interface (CLI) for developing and interacting with the Velor blockchain
# ...
```

## Change guide

Note: This guide is for developers who are trying to update the Velor homebrew formula.

You can get the latest formula here: https://github.com/Homebrew/homebrew-core/blob/master/Formula/a/velor.rb

Copy the `velor.rb` file to your `homebrew` `formula` directory. For example, on macOS with an M1, this will likely be:

```bash
/opt/homebrew/Library/Taps/homebrew/homebrew-core/Formula
```

### Development

After you've copied `velor.rb` to your local `homebrew` `formula` directory, you can modify it and use the commands
below for testing.

```bash
# On Mac M1, homebrew formulas are located locally at
/opt/homebrew/Library/Taps/homebrew/homebrew-core/Formula

# Before submitting changes run
brew audit --new-formula velor      # For new formula
brew audit velor --strict --online
brew install velor
brew test velor

# For debugging issues during the installation process you can do
brew install velor --interactive    # Interactive, gives you access to the shell
brew install velor -d               # Debug mode

# Livecheck
brew livecheck --debug velor
```

### Committing changes

Once you have audited and tested your brew formula using the commands above, make sure you:

1. Commit your changes to `velor-core` in `crates/velor/homebrew`.
2. Fork the Homebrew Core repository
   per [How to Open a Homebrew Pull Request](https://docs.brew.sh/How-To-Open-a-Homebrew-Pull-Request#formulae-related-pull-request).
3. Create a PR on the [Homebrew Core](https://github.com/Homebrew/homebrew-core/pulls) repo with your changes.

## Velor.rb structure overview

### Header

```ruby
class Velor < Formula
  desc "Layer 1 blockchain built to support fair access to decentralized assets for all"
  homepage "https://velorlabs.com/"
  url "https://github.com/velor-chain/velor-core/archive/refs/tags/velor-cli-v1.0.3.tar.gz"
  sha256 "670bb6cb841cb8a65294878af9a4f03d4cba2a598ab4550061fed3a4b1fe4e98"
  license "Apache-2.0"
  ...
```

### Bottles

[Bottles](https://docs.brew.sh/Bottles#pour-bottle-pour_bottle) are precompiled binaries. This way people don't need to
compile from source every time.

> Bottles for homebrew/core formulae are created by [Brew Test Bot](https://docs.brew.sh/Brew-Test-Bot) when a pull
> request is submitted. If the formula builds successfully on each supported platform and a maintainer approves the
> change, [Brew Test Bot](https://docs.brew.sh/Brew-Test-Bot) updates its bottle do block and uploads each bottle to
> GitHub Packages.

```ruby
  ...
  # IMPORTANT: These are automatically generated, you DO NOT need to add these manually, I'm adding them here as an example
  bottle do
    sha256 cellar: :any_skip_relocation, arm64_ventura:  "40434b61e99cf9114a3715851d01c09edaa94b814f89864d57a18d00a8e0c4e9"
    sha256 cellar: :any_skip_relocation, arm64_monterey: "edd6dcf9d627746a910d324422085eb4b06cdab654789a03b37133cd4868633c"
    sha256 cellar: :any_skip_relocation, arm64_big_sur:  "d9568107514168afc41e73bd3fd0fc45a6a9891a289857831f8ee027fb339676"
    sha256 cellar: :any_skip_relocation, ventura:        "d7289b5efca029aaa95328319ccf1d8a4813c7828f366314e569993eeeaf0003"
    sha256 cellar: :any_skip_relocation, monterey:       "ba58e1eb3398c725207ce9d6251d29b549cde32644c3d622cd286b86c7896576"
    sha256 cellar: :any_skip_relocation, big_sur:        "3e2431a6316b8f0ffa4db75758fcdd9dea162fdfb3dbff56f5e405bcbea4fedc"
    sha256 cellar: :any_skip_relocation, x86_64_linux:   "925113b4967ed9d3da78cd12745b1282198694a7f8c11d75b8c41451f8eff4b5"
  end
  ...
```

### Livecheck

[Brew livecheck](https://docs.brew.sh/Brew-Livecheck) uses strategies to find the newest version of a formula or cask’s
software by checking upstream. The strategy used below checks for all `velor-cli-v<SEMVER>` tags for `velor-core`. The
regex ensures that releases for other, non-CLI builds are not factored into livecheck.

Livecheck is run on a schedule with BrewTestBot and will update the bottles automatically on a schedule to ensure
they're up to date. For more info on how BrewTestBot and brew livecheck works, please see
the [How does BrewTestBot work and when does it update formulae?](https://github.com/Homebrew/discussions/discussions/3083)
discussion.

```ruby
...
  # This livecheck scans the releases folder and looks for all releases
  # with matching regex of href="<URL>/tag/velor-cli-v<SEMVER>". This
  # is done to automatically check for new release versions of the CLI.
  livecheck do
    url :stable
    regex(/^velor-cli[._-]v?(\d+(?:\.\d+)+)$/i)
  end
...
```

To run livecheck for testing, we recommend including the `--debug` argument:

```bash
brew livecheck --debug velor
```

### Depends on and installation

- `depends_on` is for specifying
  other [homebrew formulas as dependencies](https://docs.brew.sh/Formula-Cookbook#specifying-other-formulae-as-dependencies).
- Currently, we use v1.64 of Rust, as specified in the `Cargo.toml` file of the project. If we were to use the latest
  stable build of Rust
  going forward, we would modify the formula slightly. See the comments below for more details.

```ruby
  # Installs listed homebrew dependencies before Velor installation
  # Dependencies needed: https://velor.dev/cli-tools/build-velor-cli
  # See scripts/dev_setup.sh in velor-core for more info
  depends_on "cmake" => :build
  depends_on "rustup-init" => :build
  uses_from_macos "llvm" => :build

  on_linux do
    depends_on "pkg-config" => :build
    depends_on "zip" => :build
    depends_on "openssl@3"
    depends_on "systemd"
  end

  # Currently must compile with the same rustc version specified in the
  # root Cargo.toml file of velor-core (currently it is pegged to Rust 
  # v1.64). In the future if it becomes compatible with the latest Rust
  # toolchain, we can remove the use of rustup-init, replacing it with a 
  # depends_on "rust" => :build
  # above and build the binary without rustup as a dependency
  #
  # Uses build_cli_release.sh for creating the compiled binaries.
  # This drastically reduces their size (ie. 2.2 GB on Linux for release
  # build becomes 40 MB when run with opt-level = "z", strip, lto, etc).
  # See cargo.toml [profile.cli] section for more details
  def install
    system "#{Formula["rustup-init"].bin}/rustup-init",
      "-qy", "--no-modify-path", "--default-toolchain", "1.64"
    ENV.prepend_path "PATH", HOMEBREW_CACHE/"cargo_cache/bin"
    system "./scripts/cli/build_cli_release.sh", "homebrew"
    bin.install "target/cli/velor"
  end
```

### Tests

To conduct tests, run:

```bash
brew test velor
```

The current test generates a new key via the Velor CLI and ensures the shell output matches the filename(s) for that
key.

```ruby
  ...
  test do
    assert_match(/output.pub/i, shell_output("#{bin}/velor key generate --output-file output"))
  end
  ...
```

## Supporting resources

- To view other Homebrew-related FAQs or ask questions yourself, visit
  the [discussions board](https://github.com/orgs/Homebrew/discussions).
- For similar Rust-related build examples, we recommend:
    - [`rustfmt.rb`](https://github.com/Homebrew/homebrew-core/blob/master/Formula/rustfmt.rb)
- Finally, note these key Homebew guides:
    - [Homebrew Formula Cookbook](https://docs.brew.sh/Formula-Cookbook)
    - [Creating and Running Your Own Homebrew Tap - Rust Runbook](https://publishing-project.rivendellweb.net/creating-and-running-your-own-homebrew-tap/)
