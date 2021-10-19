// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

import type { Configuration } from './configuration';
import * as fs from 'fs';
import * as vscode from 'vscode';
import * as lc from 'vscode-languageclient';
import { log } from './log';

/** Information passed along to each VS Code command defined by this extension. */
export class Context {
    private constructor(
        private readonly extensionContext: Readonly<vscode.ExtensionContext>,
        readonly configuration: Readonly<Configuration>,
    ) { }

    static create(
        extensionContext: Readonly<vscode.ExtensionContext>,
        configuration: Readonly<Configuration>,
    ): Context | Error {
        if (!fs.existsSync(configuration.serverPath)) {
            return new Error(`command '${configuration.serverPath}' could not be found.`);
        }
        return new Context(extensionContext, configuration);
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
        this.extensionContext.subscriptions.push(disposable);
    }

    /**
     * Configures and starts the client that interacts with the language server.
     *
     * The "client" is an object that sends messages to the language server, which in Move's case is
     * the `move-analyzer` executable. Unlike registered extension commands such as
     * `move-analyzer.serverVersion`, which are manually executed by a VS Code user via the command
     * palette or menu, this client sends many of its messages on its own (for example, when it
     * starts, it sends the "initialize" request).
     *
     * To read more about the messages sent and responses received by this client, such as
     * "initialize," read [the Language Server Protocol specification](https://microsoft.github.io/language-server-protocol/specifications/specification-current/#initialize).
     **/
    startClient(): void {
        const executable: lc.Executable = {
            command: this.configuration.serverPath,
        };
        const serverOptions: lc.ServerOptions = {
            run: executable,
            debug: executable,
        };

        // The vscode-languageclient module reads a configuration option named
        // "<extension-name>.trace.server" to determine whether to log messages. If a trace output
        // channel is specified, these messages are printed there, otherwise they appear in the
        // output channel that it automatically created by the `LanguageClient` (in this extension,
        // that is 'Move Language Server'). For more information, see:
        // https://code.visualstudio.com/api/language-extensions/language-server-extension-guide#logging-support-for-language-server
        const traceOutputChannel = vscode.window.createOutputChannel(
            'Move Analyzer Language Server Trace',
        );
        const clientOptions: lc.LanguageClientOptions = {
            documentSelector: [{ scheme: 'file', language: 'move' }],
            traceOutputChannel,
        };

        const client = new lc.LanguageClient(
            'move-analyzer',
            'Move Language Server',
            serverOptions,
            clientOptions,
        );
        log.info('Starting client...');
        const disposable = client.start();
        this.extensionContext.subscriptions.push(disposable);
    }
}
