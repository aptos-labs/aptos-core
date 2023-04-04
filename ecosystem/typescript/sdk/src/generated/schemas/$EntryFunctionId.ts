/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
export const $EntryFunctionId = {
    type: 'string',
    description: `Entry function id is string representation of a entry function defined on-chain.

    Format: \`{address}::{module name}::{function name}\`

    Both \`module name\` and \`function name\` are case-sensitive.
    `,
} as const;
