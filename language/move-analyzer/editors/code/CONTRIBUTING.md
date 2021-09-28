# How to contribute to the move-analyzer Visual Studio Code extension

## Installing dependencies

To begin contributing to the VS Code extension for Move, you'll need to have some prerequisite technologies installed:

1. Visual Studio Code itself. To install, follow the directions on [the VS Code website](https://code.visualstudio.com).
2. Node.js and its package manager, NPM. To install, follow the directions on [the Node.js website](https://nodejs.org/en/).
3. The Node module dependencies this extension relies upon. To install, make sure you have Node.js and NPM installed, then open your favorite terminal app. Navigate to the extension's directory and install its dependencies using `npm install`. In short: `cd language/move-analyzer/editors/code && npm install`.

> If you'll be using [VS Code Remote Development](https://code.visualstudio.com/docs/remote/remote-overview) to connect to a computer over SSH, install Node.js on the computer you'll be connecting to.

## Running tests on the command line using `package.json` scripts

The best way to confirm that your dependency installation succeeded is by running the tests:

```sh
npm run pretest
npm run test
```

This executes the `"pretest"` and `"test"` "scripts" from thr `package.json` in the extension's directory. This `package.json` file defines not only information about the extension, such as its name and a description, but also "scripts": aliases for commands that can be run with `npm run`.

Currently, the scripts focus on compiling the project's [TypeScript](https://www.typescriptlang.org) into JavaScript (VS Code extensions run in a JavaScript environment).

## Launching VS Code with the extension installed from source

Running the extension's tests from the command-line is cumbersome because the test environment insists no other instance of VS Code is running while the tests run. Instead, to launch VS Code with the extension installed from source:

1. Open this repository in VS Code.
2. Open the "Run and Debug" view: open the command palette (keyboard shortcut `⇧⌘P` on macOS) and type in "View: Show Run and Debug".
3. From the pull-down menu in the Run and Debug view, select "Launch with Extension" and click the green arrow button. This will launch a new VS Code window that has the `move-analyzer` extension installed. If you open a `.move` file, it will be highlighted according to the any changes you've made in your local checkout of this repository.
4. Once you've launched the extension, in the original VS Code window you'll see a small box with pause, refresh, and stop buttons. Clicking these allows you to quickly refresh the extension window with your latest changes, or quit the window.

To test the Move TextMate grammar, open a file with a `.move` file extension and use the command palette to run the "Developer: Inspect Editor Tokens and Scopes" command. This displays a large window that shows what token is detected below the typing cursor in the VS Code window.

## Running the tests from within VS Code

You can run the extensions tests from within the "Run and Debug" view:

1. Open this repository in VS Code.
2. Open the "Run and Debug" view: open the command palette (keyboard shortcut `⇧⌘P` on macOS) and type in "View: Show Run and Debug".
3. Toward the bottom of the Run and Debug view, you should see an option to enable breakpoints on "Uncaught Exceptions." Clicking the check-box to enable breakpoints will make test failures easier to debug.
4. From the pull-down menu in the Run and Debug view, select "VS Code Tokenizer Tests" and click the green arrow button to run the tests. Should they fail, you will see a stack trace.
