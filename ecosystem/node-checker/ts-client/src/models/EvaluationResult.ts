/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */

export type EvaluationResult = {
    /**
     * Headline of the evaluation, e.g. "Healthy!" or "Metrics missing!".
     */
    headline: string;
    /**
     * Score out of 100.
     */
    score: number;
    /**
     * Explanation of the evaluation.
     */
    explanation: string;
    /**
     * Name of the evaluator where the evaluation came from, e.g. state_sync_version.
     */
    evaluator_name: string;
    /**
     * Category of the evaluator where the evaluation came from, e.g. state_sync.
     */
    category: string;
    /**
     * Links that might help the user fix a potential problem.
     */
    links: Array<string>;
};

