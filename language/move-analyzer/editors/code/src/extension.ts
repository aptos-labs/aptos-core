// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

import { Configuration } from './configuration';
import { Context } from './context';
import * as child_process from 'child_process';
import * as vscode from 'vscode';

/**
 * An extension command that displays the version of the server that this extension
 * interfaces with.
 */
async function serverVersion(context: Readonly<Context>): Promise<void> {
    const version = child_process.spawnSync(
        context.configuration.serverPath, ['--version'], { encoding: 'utf8' },
    );
    if (version.stdout) {
        await vscode.window.showInformationMessage(version.stdout);
    } else if (version.error) {
        await vscode.window.showErrorMessage(
            `Could not execute move-analyzer: ${version.error.message}.`,
        );
    } else {
        await vscode.window.showErrorMessage(
            `A problem occurred when executing '${context.configuration.serverPath}'.`,
        );
    }
}

/**
 * The entry point to this VS Code extension.
 *
 * As per [the VS Code documentation on activation
 * events](https://code.visualstudio.com/api/references/activation-events), "an extension must
 * export an `activate()` function from its main module and it will be invoked only once by
 * VS Code when any of the specified activation events [are] emitted."
 *
 * Activation events for this extension are listed in its `package.json` file, under the key
 * `"activationEvents"`.
 */
export function activate(extensionContext: Readonly<vscode.ExtensionContext>): void {
    const configuration = new Configuration();
    const context = Context.create(extensionContext, configuration);
    if (context instanceof Error) {
        void vscode.window.showErrorMessage(
            `Could not activate move-analyzer: ${context.message}.`,
        );
        return;
    }

    context.registerCommand('serverVersion', serverVersion);
}
