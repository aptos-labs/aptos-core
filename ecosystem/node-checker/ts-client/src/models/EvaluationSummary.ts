/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */

import type { EvaluationResult } from './EvaluationResult';

export type EvaluationSummary = {
    /**
     * Results from all the evaluations NHC ran.
     */
    evaluation_results: Array<EvaluationResult>;
    /**
     * An aggeregated summary (method TBA).
     */
    summary_score: number;
    /**
     * An overall explanation of the results.
     */
    summary_explanation: string;
};

