
# aptos-move-analyzer
**Table of Contents**
* [Introduction](#Introduction)
* [Installation](#Installation)
* [Features](#Features)
* [Support](#Support)


## Introduction <span id="Introduction">
The **aptos-move-analyzer** is a Visual Studio Code plugin for **Aptos Move** language developed by [MoveBit](https://movebit.xyz). Although this is an alpha release, it has many useful features, such as **highlight, autocomplete, go to definition/references**, and so on.

## Installation <span id="Installation">

**Note**:

1.If you already have installed *move-analyzer* or *sui-move-analyzer*, please disable them before installing **aptos-move-analyzer**, because it may have some conflicts.

2.You need to install Aptos CLI refer as https://aptos.web3doc.top/cli-tools/aptos-cli-tool/install-aptos-cli before install `aptos-move-analyzer`.

### How to Install (Must Read)
The `aptos-move-analyzer` Visual Studio Code extension works via two components: the `aptos-move-analyzer language server` and the extension itself. Below are two steps that describe how to install all of them.

### 1. Installing the `aptos-move-analyzer language server`<span id="Step1">
`aptos-move-analyzer language server` may be installed in one of two ways:

#### A. Download the precompiled binaries for the aptos-move-analyzer language server(Recommended)

```Windows```  
> Download [aptos-move-analyzer-win-installer-v0.1.4.msi](https://github.com/movebit/move/releases/tag/aptos-move-analyzer-release-v0.1.4), and proceed with the installation. This installation program will automatically add the path of `aptos-move-analyzer` to the **PATH** environment variable.

```MacOS & Ubuntu```
 
 > 1.Download binary files for the corresponding platform from [aptos-move-analyzer-releases-pages](https://github.com/movebit/move/releases/tag/aptos-move-analyzer-release-v0.1.4).
 >
 > 2.Rename it to `aptos-move-analyzer`. 
 > 
 > 3.Make sure `aptos-move-analyzer` can be found in your **PATH** environment.

 After completing the above steps, **restart** VSCode.

 #### B. Use Cargo
 
The `aptos-move-analyzer` language server is a Rust program, so we suggest installing it via `cargo`. If you haven't installed the Rust toolchain, you can install [Rustup](https://rustup.rs/), which will install the latest stable Rust toolchain including `cargo`.

**Execute the below command to install `aptos_move_analyzer`**
```
cargo install --git https://github.com/movebit/move --branch aptos-move-analyzer aptos-move-analyzer
```
The installation may take some time, often several minutes. After installation, the `aptos-move-analyzer` program is in your `cargo` binary directory. On macOS and Linux, this directory is usually `~/.cargo/bin`. You should make sure this location is in your `PATH` environment variable via `export PATH="$PATH:~/.cargo/bin"` .

To confirm that you've installed the language server program successfully, execute
`aptos-move-analyzer --version` on the command line. You should see the output `aptos-move-analyzer version number(0.1.0)`.
If you don't see it, check the troubleshooting section at the end.

After completing the above steps, **restart** VSCode.

### 2. Installing the `aptos-move-analyzer` Visual Studio Code extension

1. Open a new window in any Visual Studio Code application version 1.55.2 or greater.
2. Open the command palette (`⇧⌘P` on macOS, or use the menu item *View > Command Palette...*) and type **Extensions: Install Extensions**. This will open a panel named *Extensions* in the sidebar of your Visual Studio Code window.
3. In the search bar labeled *Search Extensions in Marketplace*, type **aptos-move-analyzer**. The aptos-move-analyzer extension should appear in the list below the search bar. Click **Install**.
4. Open any Aptos Move project directory(where the Move.toml is located), and open or create files that end in `.move`, you should see that keywords and types appear in different colors, and you can try other features.

After completing the above steps, **restart** VSCode.

### Troubleshooting
Please note: If you don't see the version number, you can refer to the troubleshooting section."

#### [1] cannot find the `aptos-move-analyzer` program
##### 1) windows
If you are installing this extension on a Windows system and have followed the steps in Section 1.A by running the windows-installer.msi, but executing `aptos-move-analyzer --version` in the command line doesn't find the `aptos-move-analyzer` program, the issue may be that VSCode cannot locate the configured environment variables. You can try the following:

   1. Restart VSCode and install the `aptos-move-analyzer` VSCode extension.
   2. In the Windows system settings, find the user environment variable `PATH`. Look for an entry ending with `MoveBit\aptos-move-analyzer\`, and copy it.
   3. Open the extension settings for `aptos-move-analyzer` in the VSCode extension store. In the `aptos-move-analyzer > server:path` entry, add the path ending with `MoveBit\aptos-move-analyzer\aptos-move-analyzer.exe` . The final result may look like: `C:\Users\YourUserName\AppData\Local\Apps\MoveBit\aptos-move-analyzer\aptos-move-analyzer.exe`
   4. Restart a terminal and try running `aptos-move-analyzer --version` in the command line again.

##### 2) mac & linux
If you see an error message *language server executable `aptos-move-analyzer` could not be found* in the
bottom-right of your Visual Studio Code screen when opening a Move file, it means that the
`aptos-move-analyzer` executable could not be found in your `PATH`. You may try the following:

1. Confirm that invoking `aptos-move-analyzer --version` in a command line terminal prints out
   `aptos-move-analyzer version number`. If it doesn't, then retry the instructions in **[step 1]**. If it
   does successfully print this output, try closing and re-opening the Visual Studio Code
   application, as it may not have picked up the update to your `PATH`.
2. If you installed the `aptos-move-analyzer` executable to a different location that is outside of your
   `PATH`, then you may have the extension look at this location by using the the Visual Studio Code
   settings (`⌘,` on macOS, or use the menu item *Code > Preferences > Settings*). Search for the
   `aptos-move-analyzer.server.path` setting, and set it to the location of the `aptos-move-analyzer` language
   server you installed.
3. If you're using it in MacOS, you may meet the error `Macos cannot verify if this app contains malicious software`, you need to add support for `aptos-move-analyzer` in the system settings Program Trust.

#### [2] analyzer not work

##### A.Need Move.toml

Open a Move source file (a file with a .move file extension) and if the opened Move source file is located within a buildable project (a Move.toml file can be found in one of its parent directories), the following advanced features will be available:

  - compiler diagnostics
  - go to definition
  - go to references
  - type on hover
  - autocomplete
  - outline view
  - generate spec
  - ...

Therefore, the Move.toml file must be found in the project directory for the plug-in's functionality to take effect.

In addition, if you have already opened the move project before, the installed plug-in will not take effect in time. You need to reopen the vscode window and open the move project code again before the plug-in is activated. 

##### B. Need Compile Project with Move.toml

When you first open a project, there will be some dependencies (configured in Move.toml) that need to be downloaded, so you need to run `aptos move compile` command first to build the project. During the build process, the dependencies will be downloaded. Once all the dependencies for the project have been downloaded, aptos-move-analyzer can properly parse the dependencies and project source code.

## Features <span id="Features">

Here are some of the features of the aptos-move-analyzer Visual Studio Code extension. To see them, open a
Move source file (a file with a `.move` file extension) and:

- See Move keywords and types highlighted in appropriate colors.
- As you type, Move keywords will appear as completion suggestions.
- If the opened Move source file is located within a buildable project (a `Move.toml` file can be
  found in one of its parent directories), the following advanced features will also be available:
  - compiler diagnostics
  - aptos commands line tool(you need install Aptos Client CLI locally)
  - aptos project template
  - go to definition
  - go to references
  - type on hover
  - inlay hints
  - linter for move file
  - ...

## Support <span id="Support">

1.If you find any issues, please report a GitHub issue to the [movebit/move-analyzer-issue](https://github.com/movebit/move-analyzer-issue) repository to get help.

2.Welcome to the developer discussion group as well: [MoveAnalyzer](https://t.me/moveanalyzer). 
