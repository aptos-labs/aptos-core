# move-analyzer

Provides language support for the Move programming language.

Currently, this means a basic grammar and language configuration for Move (`.move`) that enables
syntax highlighting, commenting/uncommenting, and other basic language features in Move files.

For information about Move, Diem, these projects' licenses, their contributor guides,
and more, visit [the Diem repository](https://github.com/diem/diem).

## How to Use

1. Open a new window in any Visual Studio Code application version 1.55.2 or greater.
2. Open the command palette (`⇧⌘P` on macOS, or use the menu item "View > Command Palette...") and type in `"Extensions: Install Extensions"`. This will open a panel named "Extensions" in the sidebar of your Visual Studio Code window.
3. In the search bar labeled "Search Extensions in Marketplace," type in "move-analyzer". The move-analyzer extension should appear in the list below the search bar. Click "Install".
4. Open any file that ends in `.move` (or, create a new file, click on "Select a language," and choose the "Move" language). As you type, you should see that keywords and types appear in different colors.

## Features

Here are some of the features of the move-analyzer Visual Studio Code extension. Open a file with a `.move` file extension, and:

* See Move keywords and types highlighted in appropriate colors.
* Comment and un-comment lines of code using the `⌘/` shortcut on macOS (or the menu command "Edit > Toggle Line Comment").
* Place your cursor on a delimiter, such as `<`, `(`, or `{`, and its corresponding delimiter -- `>`, `)`, or `}` -- will be highlighted.
