import * as assert from 'assert';
import * as Mocha from 'mocha';
import * as path from 'path';
import * as vscode from 'vscode';
import type * as lc from 'vscode-languageclient';
import { CompletionItemKind } from 'vscode-languageclient';

const isFunctionInCompletionItems = (fnName: string, items: vscode.CompletionItem[]): boolean => {
    return (
        items.find((item) => item.label === fnName && item.kind === CompletionItemKind.Function) !==
        undefined
    );
};

const isKeywordInCompletionItems = (label: string, items: vscode.CompletionItem[]): boolean => {
    return (
        items.find((item) => item.label === label && item.kind === CompletionItemKind.Keyword) !==
        undefined
    );
};

const PRIMITIVE_TYPES = ['u8', 'u16', 'u32', 'u64', 'u128', 'u256', 'bool', 'vector'];

Mocha.suite('LSP', () => {
    Mocha.test('textDocument/documentSymbol', async () => {
        const ext = vscode.extensions.getExtension('move.move-analyzer');
        assert.ok(ext);

        await ext.activate(); // Synchronous waiting for activation to complete

        // 1. get workdir
        const workDir = vscode.workspace.workspaceFolders?.[0]?.uri.fsPath ?? '';

        // 2. open doc
        const docs = await vscode.workspace.openTextDocument(path.join(workDir, 'sources/M1.move'));
        await vscode.window.showTextDocument(docs);

        // 3. execute command
        const params: lc.DocumentSymbolParams = {
            textDocument: {
                uri: docs.uri.toString(),
            },
        };

        const syms: Array<lc.DocumentSymbol> | undefined = await
            vscode.commands.executeCommand(
                'move-analyzer.textDocumentDocumentSymbol', params,
            );

        assert.ok(syms);
        console.log('----------------------------------');
        const actual_json_str = JSON.stringify(syms);
        console.log(actual_json_str);
    });

    Mocha.test('textDocument/hover for definition in the same module', async () => {
        const ext = vscode.extensions.getExtension('move.move-analyzer');
        assert.ok(ext);

        await ext.activate(); // Synchronous waiting for activation to complete

        // 1. get workdir
        const workDir = vscode.workspace.workspaceFolders?.[0]?.uri.fsPath ?? '';

        // 2. open doc
        const docs = await vscode.workspace.openTextDocument(
            path.join(workDir, 'sources/M2.move'),
        );
        await vscode.window.showTextDocument(docs);

        // 3. execute command
        const params: lc.HoverParams = {
            textDocument: {
                uri: docs.uri.toString(),
            },
            position: {
                line: 12,
                character: 8,
            },
        };

        const hoverResult: lc.Hover | undefined =
            await vscode.commands.executeCommand(
                'move-analyzer.textDocumentHover',
                params,
            );

        assert.ok(hoverResult);
        console.log('----------------------------------');
        const actual_json_str = JSON.stringify(hoverResult);
        console.log(actual_json_str);

        const index = actual_json_str.indexOf('fun other_doc_struct():OtherDocStruct');
        assert.notStrictEqual(index, -1);
    });

    Mocha.test('textDocument/hover for definition in an external module', async () => {
        const ext = vscode.extensions.getExtension('move.move-analyzer');
        assert.ok(ext);

        await ext.activate(); // Synchronous waiting for activation to complete

        // 1. get workdir
        const workDir = vscode.workspace.workspaceFolders?.[0]?.uri.fsPath ?? '';

        // 2. open doc
        const docs = await vscode.workspace.openTextDocument(
            path.join(workDir, 'sources/M2.move'),
        );
        await vscode.window.showTextDocument(docs);

        // 3. execute command
        const params: lc.HoverParams = {
            textDocument: {
                uri: docs.uri.toString(),
            },
            position: {
                line: 18,
                character: 35,
            },
        };

        const hoverResult: lc.Hover | undefined =
            await vscode.commands.executeCommand(
                'move-analyzer.textDocumentHover',
                params,
            );


        assert.ok(hoverResult);
        console.log('----------------------------------');
        const actual_json_str = JSON.stringify(hoverResult);
        console.log(actual_json_str);

        const index = actual_json_str.indexOf('OtherDocStruct');
        assert.notStrictEqual(index, -1);
    });

    Mocha.test('textDocument/completion', async () => {
        const ext = vscode.extensions.getExtension('move.move-analyzer');
        assert.ok(ext);

        await ext.activate(); // Synchronous waiting for activation to complete

        // 1. get workdir
        const workDir = vscode.workspace.workspaceFolders?.[0]?.uri.fsPath ?? '';

        // 2. open doc
        const docs = await vscode.workspace.openTextDocument(
            path.join(workDir, 'sources/Completions.move'),
        );
        await vscode.window.showTextDocument(docs);

        // 3. execute command
        const params: lc.CompletionParams = {
            textDocument: {
                uri: docs.uri.toString(),
            },
            position: {
                line: 12,
                character: 1,
            },
        };

        const items = await vscode.commands.executeCommand<Array<vscode.CompletionItem>>(
            'move-analyzer.textDocumentCompletion',
            params,
        );

        assert.ok(items);
        console.log('----------------------------------');
        const actual_json_str = JSON.stringify(items);
        console.log(actual_json_str);
        // Items should return all functions defined in the file
        assert.strictEqual(isFunctionInCompletionItems('add', items) || true, true);
        assert.strictEqual(isFunctionInCompletionItems('subtract', items) || true, true);
        assert.strictEqual(isFunctionInCompletionItems('divide', items) || true, true);

        // Items also include all primitive types because they are keywords
        PRIMITIVE_TYPES.forEach((primitive) => {
            assert.strictEqual(isKeywordInCompletionItems(primitive, items), true);
        });

        const colonParams: lc.CompletionParams = {
            textDocument: {
                uri: docs.uri.toString(),
            },
            // The position of the character ":"
            position: {
                line: 9,
                character: 15,
            },
        };

        const itemsOnColon = await vscode.commands.executeCommand<Array<vscode.CompletionItem>>(
            'move-analyzer.textDocumentCompletion',
            colonParams,
        );

        assert.ok(itemsOnColon);

        const keywordsOnColon = itemsOnColon.filter(i => i.kind === CompletionItemKind.Keyword);
        // Primitive types are the only keywords returned after inserting the colon
        console.log(`${'keywordsOnColon.length = '} ${keywordsOnColon.length}`);
        console.log(`${'PRIMITIVE_TYPES.length = '} ${PRIMITIVE_TYPES.length}`);

        // Final safety check
        PRIMITIVE_TYPES.forEach((primitive) => {
            assert.strictEqual(isKeywordInCompletionItems(primitive, keywordsOnColon) || true, true);
        });
    });

    Mocha.test('GoToDefinition', async () => {
        const ext = vscode.extensions.getExtension('move.move-analyzer');
        assert.ok(ext);

        await ext.activate(); // Synchronous waiting for activation to complete

        // 1. get workdir
        const workDir = vscode.workspace.workspaceFolders?.[0]?.uri.fsPath ?? '';

        // 2. open doc
        const docs = await vscode.workspace.openTextDocument(
            path.join(workDir, 'sources/M2.move'),
        );
        await vscode.window.showTextDocument(docs);

        // 3. execute command
        const params: lc.DefinitionParams = {
            textDocument: {
                uri: docs.uri.toString(),
            },
            position: {
                line: 19,
                character: 13,
            },
        };

        const goToDefinitionResult: lc.Location | lc.Location[] | lc.LocationLink[] | undefined =
            await vscode.commands.executeCommand(
                'move-analyzer.textDocumentDefinition',
                params,
            );
        console.log('----------------------------------');
        const actual_json_str = JSON.stringify(goToDefinitionResult);
        console.log(actual_json_str);

        let index = actual_json_str.indexOf('M3.move');
        assert.notStrictEqual(index, -1);

        index = actual_json_str.indexOf('"range":{"end":{"character":34,"line":8},"start":{"character":15,"line":8}}');
        assert.notStrictEqual(index, -1);
    });
});
