// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

import type { Configuration } from './configuration';
import * as fs from 'fs';
import * as vscode from 'vscode';

/** Information passed along to each VS Code command defined by this extension. */
export class Context {
    private constructor(
        private readonly extension: Readonly<vscode.ExtensionContext>,
        readonly configuration: Readonly<Configuration>,
    ) { }

    static create(
        extension: Readonly<vscode.ExtensionContext>,
        configuration: Readonly<Configuration>,
    ): Context | Error {
        if (!fs.existsSync(configuration.serverPath)) {
            return new Error(`command '${configuration.serverPath}' could not be found.`);
        }
        return new Context(extension, configuration);
    }

    /**
     * Registers the given command with VS Code.
     *
     * "Registering" the function means that the VS Code machinery will execute it when the command
     * with the given name is requested by the user. The command names themselves are specified in
     * this extension's `package.json` file, under the key `"contributes.commands"`.
     */
    registerCommand(
        name: Readonly<string>,
        command: (context: Readonly<Context>) => Promise<void>,
    ): void {
        const disposable = vscode.commands.registerCommand(`move-analyzer.${name}`, async () => {
            return command(this);
        });
        this.extension.subscriptions.push(disposable);
    }
}
