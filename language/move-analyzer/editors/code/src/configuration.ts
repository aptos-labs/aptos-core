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

    /** A string representation of the configured values, for logging purposes. */
    toString(): string {
        return JSON.stringify(this.configuration);
    }

    /** The path to the move-analyzer executable. */
    get serverPath(): string {
        const defaultName = 'move-analyzer';
        const path = this.configuration.get<string>('server.path', defaultName);
        if (path.length === 0) {
            // The default value of the `server.path` setting is 'move-analyzer'.
            // A user may have over-written this default with an empty string value, ''.
            // An empty string cannot be an executable name, so instead use the default.
            return defaultName;
        }
        if (path.startsWith('~/')) {
            return os.homedir() + path.slice('~'.length);
        }
        return path;
    }
}
