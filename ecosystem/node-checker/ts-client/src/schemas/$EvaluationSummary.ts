/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
export const $EvaluationSummary = {
    properties: {
        evaluation_results: {
            type: 'array',
            contains: {
                type: 'EvaluationResult',
            },
            isRequired: true,
        },
        summary_score: {
            type: 'number',
            description: `An aggeregated summary (method TBA).`,
            isRequired: true,
            format: 'uint8',
        },
        summary_explanation: {
            type: 'string',
            description: `An overall explanation of the results.`,
            isRequired: true,
        },
    },
} as const;
