# move-analyzer

Provides language support for the Move programming language.

Currently, this means a basic grammar and language configuration for Move (`.move`) that enables
syntax highlighting, commenting/uncommenting, simple context-unaware completion suggestions while
typing, and other basic language features in Move files.

For information about Move, Diem, these projects' licenses, their contributor guides, links to our
Discord server, and more, visit [the Diem repository](https://github.com/diem/diem).

## How to Install

The move-analyzer Visual Studio Code extension works via two components: the extension itself, and
the `move-analyzer` language server.

### 1. Installing the `move-analyzer` language server<span id="Step1">

The `move-analyzer` language server is a Rust program that is part of
[the Diem repository](https://github.com/diem/diem). It may be installed in one of two ways:

1. You may clone [the Diem repository](https://github.com/diem/diem) yourself and build
   `move-analyzer` from its source code. This is recommended for Diem hackathon participants, and
   Diem & Move core developers.
   1. Follow the instructions in the Move tutorial's
      [Step 0: Installation](https://github.com/diem/diem/tree/main/language/documentation/tutorial#step-0-installation).
   2. To confirm that you've built the language server program successfully, execute
      `<path_to_diem_repo>/target/debug/move-analyzer --version` on the command line. You should see
      the output `move-analyzer 0.0.0`.
2. You may use Rust's package manager `cargo` to install `move-analyzer` in your user's PATH. This
   is recommended for people who do not work on Diem & Move core.
   1. If you don't already have a Rust toolchain installed, you should install
      [Rustup](https://rustup.rs/), which will install the latest stable Rust toolchain.
   2. Invoke `cargo install --git https://github.com/diem/diem move-analyzer` to install the
      `move-analyzer` language server in your Cargo binary directory. On macOS and Linux this is
      usually `~/.cargo/bin`. You'll want to make sure this location is in your `PATH` environment
      variable.
   3. To confirm that you've installed the language server program successfully, execute
      `move-analyzer --version` on the command line. You should see the output
      `move-analyzer 0.0.0`.

### 2. Installing the move-analyzer Visual Studio Code extension

1. Open a new window in any Visual Studio Code application version 1.55.2 or greater.
2. Open the command palette (`⇧⌘P` on macOS, or use the menu item "View > Command Palette...") and
   type in `"Extensions: Install Extensions"`. This will open a panel named "Extensions" in the
   sidebar of your Visual Studio Code window.
3. In the search bar labeled "Search Extensions in Marketplace," type in "move-analyzer". The
   move-analyzer extension should appear in the list below the search bar. Click "Install".
4. Open the Visual Studio Code settings (`⌘,` on macOS, or use the menu item "Code > Preferences >
   Settings"). Search for the `move-analyzer.server.path` setting, and set it to the location of the
   `move-analyzer` language server you installed above.
   1. If you used method 1, it should exist at `<path_to_diem_repo>/target/debug/move-analyzer`.
   2. If you used method 2, it should exist in your `PATH` as `move-analyzer`. This is the default
      value, so you do not need to edit this setting.
5. Open any file that ends in `.move` (or, create a new file, click on "Select a language," and
   choose the "Move" language). As you type, you should see that keywords and types appear in
   different colors.

**Note:** If you see an error message "language server executable '/path/to/move-analyzer' could not
be found" in the bottom-right of your Visual Studio Code screen when opening a Move file, it means
that the `move-analyzer` executable does not exist at the path you specified in your
`move-analyzer.server.path` setting. Change the setting to point to the location of a
`move-analyzer` executable you built or installed in [step 1](./Step1).

## Features

Here are some of the features of the move-analyzer Visual Studio Code extension. Open a file with a
`.move` file extension, and:

- See Move keywords and types highlighted in appropriate colors.
- Comment and un-comment lines of code using the `⌘/` shortcut on macOS (or the menu command "Edit >
  Toggle Line Comment").
- Place your cursor on a delimiter, such as `<`, `(`, or `{`, and its corresponding delimiter --
  `>`, `)`, or `}` -- will be highlighted.
- As you type, Move keywords will appear as completion suggestions.
