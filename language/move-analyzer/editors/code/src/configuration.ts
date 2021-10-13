// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

import * as os from 'os';
import * as vscode from 'vscode';

/**
 * User-defined configuration values, such as those specified in VS Code settings.
 *
 * This provides a more strongly typed interface to the configuration values specified in this
 * extension's `package.json`, under the key `"contributes.configuration.properties"`.
 */
export class Configuration {
    private readonly configuration: vscode.WorkspaceConfiguration;

    constructor() {
        this.configuration = vscode.workspace.getConfiguration('move-analyzer');
    }

    /** The path to the move-analyzer executable. */
    get serverPath(): string {
        const path = this.configuration.get<string>('server.path', 'move-analyzer');
        if (path.startsWith('~/')) {
            return os.homedir() + path.slice('~'.length);
        }
        return path;
    }
}
