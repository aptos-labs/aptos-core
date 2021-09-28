// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

/**
 * This file contains what VS Code's documentation refers to as "the test runner," which
 * programmatically executes each test file in our extension. For more information, see:
 * https://code.visualstudio.com/api/working-with-extensions/testing-extension#the-test-runner-script
 */

import * as glob from 'glob';
import * as Mocha from 'mocha';
import * as path from 'path';

export async function run(): Promise<void> {
    const suite = new Mocha({
        ui: 'tdd',
        color: true,
    });

    const testsRoot = path.resolve(__dirname, '..');
    return new Promise((resolve, reject) => {
        // The test suite is composed of all files ending with '.test.js'.
        glob('**/**.test.js', { cwd: testsRoot }, (err, files: ReadonlyArray<string>) => {
            if (err) {
                return reject(err);
            }

            // Add each file to the test suite.
            files.forEach(f => suite.addFile(path.resolve(testsRoot, f)));

            // Run the test suite. Uncaught exceptions or a non-zero number of
            // test rejectures is considered a test suite rejecture.
            try {
                return suite.run(failures => {
                    if (failures > 0) {
                        reject(new Error(`${failures} tests failed.`));
                    } else {
                        resolve();
                    }
                });
            } catch (err: unknown) {
                console.error(err);
                return reject(err);
            }
        });
    });
}
