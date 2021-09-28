// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

import * as assert from 'assert';
import * as fs from 'fs';
import * as Mocha from 'mocha';
import * as path from 'path';
import { commands, Uri } from 'vscode';

/** The 'captureSyntaxTokens' command returns an array of tokens of this type. */
interface Token {
    /** The token lexeme, i.e.: the text that the token represents. */
    c: string;
    /**
     * A space-delineated string of TextMate token groups.
     * For example, "source.move comment.line.move".
     */
    t: string;
    /**
     * A mapping between VS Code theme names, and color codes for the token under those themes.
     * For example, `{ "dark_vs": "comment: #6A9955" }`.
     */
    r: Record<string, string>;
}

/**
 * Asserts that the tokens our extension generates for the given test fixture matches the
 * expectations defined in the 'colorize-results' directory.
 *
 * If a "result" file is not already defined, this creates one. Insignificant differences between
 * the extension output and the result file are ignored. If the only differences are insignificant,
 * then the result file is overwritten with the most recent extension output.
 */
function assertUnchangedTokens(fixturePath: string, done: Mocha.Done): void {
    const command = '_workbench.captureSyntaxTokens';
    commands.executeCommand(command, Uri.file(fixturePath)).then((data: unknown) => {
        const resultsPath = path.resolve(fixturePath, '..', '..', 'colorize-results');
        assert(fs.existsSync(resultsPath), `results directory '${resultsPath}' must be present`);

        const tokens = data as Array<Token>;
        const resultPath = path.join(resultsPath, path.basename(fixturePath) + '.json');
        if (fs.existsSync(resultPath)) {
            // If the result file exists, test against it.
            const previousTokens =
                JSON.parse(fs.readFileSync(resultPath).toString()) as Array<Token>;
            try {
                assert.deepStrictEqual(tokens, previousTokens);
            } catch (e: unknown) {
                // If the tokenization result is not exactly the same as what was previously in the
                // result file, the changes may or may not be significant. For example, the color
                // values for tokens are embedded into the tokenization result JSON, but these are
                // based on the VS Code theme settings -- if only these color values differ, then
                // the test should not fail (and the result file is overwritten with the new color
                // values).
                assert(Array.isArray(tokens) && Array.isArray(previousTokens));
                if (!Array.isArray(tokens) || !Array.isArray(previousTokens) ||
                    tokens.length !== previousTokens.length) {
                    // A difference in the number of tokens is a significant change.
                    throw e;
                }

                for (let i = 0; i < tokens.length; i++) {
                    const token = tokens[i];
                    const previous = previousTokens[i];
                    assert(token && previous);
                    if (token.c !== previous.c || token.t !== previous.t) {
                        // If the tokens' lexemes or groups differ, the delta is a significant one
                        // and the test should fail.
                        throw e;
                    }
                }

                // If the only deltas are insignificant ones, overwrite the result file so that
                // future test runs will be strictly equal. (Append a newline to appease linters
                // that enforce EOF newlines.)
                fs.writeFileSync(
                    resultPath,
                    JSON.stringify(tokens, null, '\t') + '\n',
                    { flag: 'w' },
                );
            }
        } else {
            // If the result file doesn't exist, create it with the result of the extension's
            // current tokenizer. (Append a newline to appease linters that enforce EOF newlines.)
            fs.writeFileSync(resultPath, JSON.stringify(tokens, null, '\t') + '\n');
        }
        done();
    }, done);
}

// A Mocha test suite composed of one test per "fixture" in the 'colorize-fixtures' directory.
Mocha.suite('colorization', () => {
    const testsRoot = path.resolve(__dirname, '..', '..', 'tests');
    const fixturesDirectory = path.join(testsRoot, 'colorize-fixtures');
    assert(fs.existsSync(fixturesDirectory),
        `fixtures directory '${fixturesDirectory}' must be present`);

    const fixtures = fs.readdirSync(fixturesDirectory);
    assert(fixtures.length > 0,
        `fixtures directory '${fixturesDirectory}' must contain at least one file`);

    fixtures.forEach(fixture => {
        Mocha.test(fixture, function(done) {
            assertUnchangedTokens(path.join(fixturesDirectory, fixture), done);
        });
    });
});
