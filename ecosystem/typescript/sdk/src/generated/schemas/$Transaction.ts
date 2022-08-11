/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
export const $Transaction = {
    type: 'one-of',
    contains: [{
        type: 'Transaction_PendingTransaction',
    }, {
        type: 'Transaction_UserTransaction',
    }, {
        type: 'Transaction_GenesisTransaction',
    }, {
        type: 'Transaction_BlockMetadataTransaction',
    }, {
        type: 'Transaction_StateCheckpointTransaction',
    }],
} as const;
