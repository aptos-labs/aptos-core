/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
export const $EvaluationResult = {
    properties: {
        headline: {
            type: 'string',
            description: `Headline of the evaluation, e.g. "Healthy!" or "Metrics missing!".`,
            isRequired: true,
        },
        score: {
            type: 'number',
            description: `Score out of 100.`,
            isRequired: true,
            format: 'uint8',
        },
        explanation: {
            type: 'string',
            description: `Explanation of the evaluation.`,
            isRequired: true,
        },
        evaluator_name: {
            type: 'string',
            description: `Name of the evaluator where the evaluation came from, e.g. state_sync_version.`,
            isRequired: true,
        },
        category: {
            type: 'string',
            description: `Category of the evaluator where the evaluation came from, e.g. state_sync.`,
            isRequired: true,
        },
        links: {
            type: 'array',
            contains: {
                type: 'string',
            },
            isRequired: true,
        },
    },
} as const;
