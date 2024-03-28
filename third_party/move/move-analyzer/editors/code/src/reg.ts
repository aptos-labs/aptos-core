// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

import type { Context } from './context';
import * as vscode from 'vscode';
import * as fs from 'fs';
import * as path from 'path';
import * as childProcess from 'child_process';

/**
 * A logger for the VS Code extension.
 *
 * Messages that are logged appear in an output channel created below that is dedicated to the
 * extension (or "client"), in the extension user's "Output View." This logger should be used for
 * messages related to VS Code and this extension, as opposed to messages regarding the language
 * server, which appear in a separate output channel.
 **/

class TraverseDirItem {
    path: string;

    is_file: boolean;

    constructor(path: string,
        is_file: boolean) {
        this.path = path;
        this.is_file = is_file;
    }
}


function workSpaceDir(): string | undefined {
    if (vscode.workspace.workspaceFolders !== undefined) {
        if (vscode.workspace.workspaceFolders[0] !== undefined) {
            const f = vscode.workspace.workspaceFolders[0].uri.fsPath;
            return f;
        }
    }
    return undefined;
}

async function serverVersion(context: Readonly<Context>): Promise<void> {
    const version = childProcess.spawnSync(
        context.configuration.serverPath,
        ['--version'],
        { encoding: 'utf8' },
    );
    if (version.stdout) {
        await vscode.window.showInformationMessage(version.stdout);
    } else if (version.error) {
        await vscode.window.showErrorMessage(
            `Could not execute aptos-move-analyzer: ${version.error.message}.`,
        );
    } else {
        await vscode.window.showErrorMessage(
            `A problem occurred when executing '${context.configuration.serverPath}'.`,
        );
    }
}

function traverseDir(dir: any, call_back: (path: TraverseDirItem) => void): void {
    fs.readdirSync(dir).forEach(file => {
        const fullPath = path.join(dir, file);
        if (fs.lstatSync(fullPath).isDirectory()) {
            call_back(new TraverseDirItem(fullPath, false));
            traverseDir(fullPath, call_back);
        } else {
            call_back(new TraverseDirItem(fullPath, true));
        }
    });
}

function get_all_move_toml_dirs(): string[] {
    const working_dir = workSpaceDir();
    if (working_dir === undefined) {
        return [];
    }
    const ret: string[] = [];
    traverseDir(working_dir, (item) => {
        if (item.is_file && item.path.endsWith('Move.toml')) {
            ret.push(item.path);
        }
    });
    return ret;
}

class TerminalManager {
    all: Map<string, vscode.Terminal | undefined>;

    constructor() {
        this.all = new Map();
    }

    alloc(typ: string, new_fun: () => vscode.Terminal): vscode.Terminal {
        const x = this.all.get(typ);
        if (x === undefined || x.exitStatus !== undefined) {
            const x = new_fun();
            this.all.set(typ, x);
            return x;
        }
        return x;
    }
}


class WorkingDir {

    private dir: string | undefined;

    constructor() {
        const working_dir = workSpaceDir();
        if (working_dir === undefined) {
            this.dir = undefined;
        }
        const x = get_all_move_toml_dirs();
        if (x.length === 1) {
            this.dir = working_dir;
        }
        this.dir = undefined;
    }

    // Change the current working dir
    set_dir(Dir: string): void {
        this.dir = Dir;
    }

    // Get the current working dir, if is undefined, return ""
    get_dir(): string {
        if (this.dir !== undefined) {
            return this.dir;
        }
        return '';
    }

    async get_use_input_working_dir(): Promise<string | undefined> {
        return vscode.window.showQuickPick(get_all_move_toml_dirs(),
            {
            }).then((x): string | undefined => {
                if (x === undefined) {
                    return undefined;
                }
                this.dir = path.parse(x).dir;
                return this.dir;
            });
    }

    async get_working_dir(): Promise<string | undefined> {
        if (this.dir !== undefined) {
            return this.dir;
        }
        return this.get_use_input_working_dir();
    }

}
const Reg = {

    /** Regist all the command for aptos framework for main.ts */
    regaptos(context: Readonly<Context>): void {
        /**
         * An extension command that displays the version of the server that this extension
         * interfaces with.
         */
        const aptos_working_dir = new WorkingDir();
        const terminalManager = new TerminalManager();
        const schemaTypes = ['ed25519', 'secp256k1', 'secp256r1'];
        const aptos_move_toml_template = 
`[package]
name = "aptos-counter"
version = "1.0.0"
authors = []

[addresses]
publisher = "0x123321"

[dev-addresses]

[dependencies.AptosFramework]
git = "https://github.com/aptos-labs/aptos-core.git"
rev = "mainnet"
subdir = "aptos-move/framework/aptos-framework"

[dev-dependencies]`;

        const aptos_module_file_template = 
`// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

module publisher::my_module {
    // Part 1: imports
    use std::signer;

    // Part 2: struct definitions
    struct CountHolder has key {
        count: u64
    }

    public fun get_count(addr: address): u64 acquires CountHolder {
        assert!(exists<CountHolder>(addr),0);
        *&borrow_global<CountHolder>(addr).count
    }

    // Part 3: entry functions
    public entry fun bump(account: signer)
    acquires CountHolder 
    {
        let addr = signer::address_of(&account);
        if(!exists<CountHolder>(addr)) {
            move_to(&account, CountHolder {
                count: 0
            })
        } else {
            let old_count = borrow_global_mut<CountHolder>(addr);
            old_count.count = old_count.count + 1;
        }
    }
}`;

        if (aptos_working_dir.get_dir() !== '') {
            void vscode.window.showInformationMessage('aptos working directory set to ' + aptos_working_dir.get_dir());
        }

        // Register handlers for VS Code commands that the user explicitly issues.
        context.registerCommand('serverVersion', serverVersion);
        // Register test button
        context.registerCommand('test_ui', (_, ...args) => {
            const cwd = args[0] as string;
            const name = args[1] as string;
            const aptos_test = terminalManager.alloc(cwd + 'test_ui', () => {
                return vscode.window.createTerminal({
                    cwd: cwd,
                    name: 'aptos test',
                });
            });
            aptos_test.show(true);
            aptos_test.sendText('aptos move test ' + name, true);
            aptos_test.show(false);
        });

        context.registerCommand('create_project', async () => {
            const working_dir = await aptos_working_dir.get_working_dir();
            const dir = await vscode.window.showSaveDialog({
                // There is a long term issue about parse()
                // use "." instead of working dir, detail in https://github.com/microsoft/vscode/issues/173687
                defaultUri: vscode.Uri.parse(working_dir!),
            });

            if (dir === undefined) {
                void vscode.window.showErrorMessage('Please input a directory');
                return;
            }
            const dir2 = dir.fsPath;
            fs.mkdirSync(dir2);
            const project_name = path.parse(dir2).base;
            const replace_name = 'my_first_package';
            fs.writeFileSync(dir2 + '/Move.toml',
                aptos_move_toml_template.toString().replaceAll(replace_name, project_name));
            fs.mkdirSync(dir2 + '/sources');
            fs.writeFileSync(dir2 + '/sources/my_module.move',
                aptos_module_file_template.replaceAll(replace_name, project_name));
        });
        context.registerCommand('move.compile', async () => {
            const working_dir = await aptos_working_dir.get_working_dir();
            if (working_dir === undefined) {
                return;
            }
            const t = terminalManager.alloc('move.compile', (): vscode.Terminal => {
                return vscode.window.createTerminal({
                    name: 'aptos move compile',
                });
            });
            t.show(true);
            t.sendText('cd ' + working_dir, true);
            t.sendText('aptos move compile', true);
        });
        context.registerCommand('move.coverage', async () => {
            const working_dir = await aptos_working_dir.get_working_dir();
            if (working_dir === undefined) {
                return;
            }
            const t = terminalManager.alloc('move.coverage', (): vscode.Terminal => {
                return vscode.window.createTerminal({
                    name: 'aptos move coverage',
                });
            });
            t.show(true);
            t.sendText('cd ' + working_dir, true);
            t.sendText('aptos move test --coverage', true);
            t.sendText('aptos move coverage summary', true);
        });
        context.registerCommand('move.test', async () => {
            const working_dir = await aptos_working_dir.get_working_dir();
            if (working_dir === undefined) {
                return;
            }
            const t = terminalManager.alloc('move.test', (): vscode.Terminal => {
                return vscode.window.createTerminal({
                    name: 'aptos move test',
                });
            });
            t.show(true);
            t.sendText('cd ' + working_dir, true); t.sendText('cd ' + working_dir, true);
            t.sendText('aptos move test', true);
        });
        context.registerCommand('move.prove', async () => {
            const working_dir = await aptos_working_dir.get_working_dir();
            if (working_dir === undefined) {
                return;
            }
            const t = terminalManager.alloc('move.prove', (): vscode.Terminal => {
                return vscode.window.createTerminal({
                    name: 'aptos move prove',
                });
            });
            t.show(true);
            t.sendText('cd ' + working_dir, true);
            t.sendText('aptos move prove', true);
        });
        context.registerCommand('key.generate', async () => {
            const working_dir = await aptos_working_dir.get_working_dir();
            if (working_dir === undefined) {
                return;
            }
            const schema = await vscode.window.showQuickPick(schemaTypes, {
                canPickMany: false, placeHolder: 'Select you schema.',
            });
            if (schema === undefined) {
                return;
            }
            const t = terminalManager.alloc('client.key.generate', (): vscode.Terminal => {
                return vscode.window.createTerminal({
                    name: 'aptos key generate',
                });
            });
            t.show(true);
            t.sendText('cd ' + working_dir, true);
            t.sendText('aptos key generate ' + schema, true);
        });
        context.registerCommand('key.extract-peer', async () => {
            const working_dir = await aptos_working_dir.get_working_dir();
            if (working_dir === undefined) {
                return;
            }
            const m = await vscode.window.showInputBox({
                placeHolder: 'Type your mnemonic phrase.',
            });
            if (m === undefined) {
                return;
            }
            const schema = await vscode.window.showQuickPick(schemaTypes, {
                canPickMany: false, placeHolder: 'Select you schema.',
            });
            if (schema === undefined) {
                return;
            }
            const t = terminalManager.alloc('client.key.extract-peer', (): vscode.Terminal => {
                return vscode.window.createTerminal({
                    name: 'aptos key extract-peer',
                });
            });
            t.show(true);
            t.sendText('cd ' + working_dir, true);
            t.sendText('aptos key extract-peer ' + m + ' ' + schema, true);
        });
        context.registerCommand('reset.working.space', async () => {
            const new_ = await aptos_working_dir.get_use_input_working_dir();
            if (new_ === undefined) {
                return;
            }
            aptos_working_dir.set_dir(new_);
            void vscode.window.showInformationMessage('aptos working directory set to ' + new_);
        });
        context.registerCommand('move.generate.spec.file', (_, ...args) => {
            interface FsPath {
                fsPath: string;
            }
            if (args.length === 0) {
                return;
            }
            const fsPath = (args[0] as FsPath).fsPath;
            if (fsPath.endsWith('.spec.move')) {
                void vscode.window.showErrorMessage('This is already a spec file');
                return;
            }
            const client = context.getClient();
            if (client === undefined) {
                return;
            }
            interface Result {
                fpath: string;
            }
            client.sendRequest<Result>('move/generate/spec/file', { 'fpath': fsPath }).then(
                (result) => {
                    void vscode.workspace.openTextDocument(result.fpath).then((a) => {
                        void vscode.window.showTextDocument(a);
                    });
                },
            ).catch((err) => {
                void vscode.window.showErrorMessage('generate failed: ' + (err as string));
            });
        });
        context.registerCommand('move.generate.spec.sel', (_, ...args) => {
            interface FsPath {
                fsPath: string;
            }
            if (args.length === 0) {
                return;
            }
            if (vscode.window.activeTextEditor === undefined) {
                return;
            }
            const line = vscode.window.activeTextEditor.selection.active.line;
            const col = vscode.window.activeTextEditor.selection.active.character;
            const fsPath = (args[0] as FsPath).fsPath;
            if (fsPath.endsWith('.spec.move')) {
                void vscode.window.showErrorMessage('This is already a spec file');
                return;
            }
            const client = context.getClient();
            if (client === undefined) {
                return;
            }
            interface Result {
                content: string;
                line: number;
                col: number;
            }

            client.sendRequest<Result>('move/generate/spec/sel', { 'fpath': fsPath, line: line, col: col }).then(
                (result) => {
                    vscode.window.activeTextEditor?.edit((e) => {
                        e.insert(new vscode.Position(result.line, result.col), result.content);
                    });
                },
            ).catch((err) => {
                void vscode.window.showErrorMessage('generate failed: ' + (err as string));
            });
        });

        context.registerCommand('goto_definition', async (_context, ...args) => {
            const loc = args[0] as { range: vscode.Range; fpath: string };
 
            const client = context.getClient();
            if (client === undefined) {
                return;
            }
            
            if (loc.range.start.line == 0 && loc.range.end.line == 0 &&
                loc.range.start.character == 0 && loc.range.end.character == 0) {
                    void vscode.window.showWarningMessage(
                        "Sorry, for goto-to-definition of Inlay-Hints, Aptos Move Analyzer only supports structs temporarily ."
                    );
            } else {
                try {
                    const document = await vscode.workspace.openTextDocument(loc.fpath);
                    await vscode.window.showTextDocument(document, { selection: loc.range, preserveFocus: false });
                } catch (error) {
                    // 处理错误
                    console.error('Error opening file:', error);
                }
               
            }
        });
    },

};

export { Reg, WorkingDir };
// X const t = await vscode.workspace.openTextDocument(loc.fpath);
// X await vscode.window.showTextDocument(t, { selection: loc.range, preserveFocus: false });

// interface Result {
            //     content: string;
            //     line: number;
            //     col: number;
            // }

            // if (loc.range.start.line == 0 && loc.range.end.line == 0 &&
            //     loc.range.start.character == 0 && loc.range.end.character == 0) {
            //         void vscode.window.showWarningMessage(
            //             "Sorry, for goto-to-definition of Inlay-Hints, Aptos Move Analyzer only supports structs temporarily ."
            //         );
            // } else {
            //     vscode.workspace.openTextDocument(loc.fpath).then(document => {
            //         // 显示文档
            //         vscode.window.showTextDocument(document, { 
            //             selection: new vscode.Range(
            //                 loc.range.start.line, 
            //                 loc.range.start.character, 
            //                 loc.range.start.line, 
            //                 loc.range.start.character
            //             ) 
            //         });
            //     });
            // }

            
            // void vscode.window.showErrorMessage(loc.fpath);
            // // void vscode.window.showErrorMessage(loc.range.start.line as string);
            // client.sendRequest<Result>('move/goto_definition', { 'fpath': loc.fpath, selection: loc.range }).then(
                
            //     (result) => {
            //         console.warn(result);
            //         vscode.window.activeTextEditor?.edit((e) => {
            //             e.insert(new vscode.Position(result.line, result.col), result.content);
            //         });
            //     },
            // ).catch((err) => {
            //     void vscode.window.showErrorMessage('generate failed: ' + (err as string));
            // });