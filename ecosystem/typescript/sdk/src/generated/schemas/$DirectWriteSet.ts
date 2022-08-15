/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
export const $DirectWriteSet = {
    properties: {
        changes: {
            type: 'array',
            contains: {
                type: 'WriteSetChange',
            },
            isRequired: true,
        },
        events: {
            type: 'array',
            contains: {
                type: 'Event',
            },
            isRequired: true,
        },
    },
} as const;
