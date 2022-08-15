/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
export const $ScriptFunctionId = {
    type: 'string',
    description: `Script function id is string representation of a script function defined on-chain.

    Format: \`{address}::{module name}::{function name}\`

    Both \`module name\` and \`function name\` are case-sensitive.
    `,
} as const;
