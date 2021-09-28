// FIXME: The vscode-test `runTests` function is implemented in TypeScript and has type annotations:
// https://github.com/microsoft/vscode-test/blob/v1.6.1/lib/runTest.ts#L86
// But I don't know where, if anywhere, those types are published, so to appease the TypeScript
// compiler's type-checker, I declare the function type in this file.
declare module '@vscode/test-electron' {
    export interface TestOptions {
        extensionDevelopmentPath?: string;
        extensionTestsPath?: string;
        // N.B.: vscode-test's definition of `TestOptions` contains many more fields:
        // https://github.com/microsoft/vscode-test/blob/v1.6.1/lib/runTest.ts#L9-L79
    }

    export function runTests(options: TestOptions): Thenable<number>;
}
