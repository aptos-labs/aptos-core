/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
export const $GenesisPayload = {
    type: 'one-of',
    description: `The writeset payload of the Genesis transaction`,
    contains: [{
        type: 'GenesisPayload_WriteSetPayload',
    }],
} as const;
