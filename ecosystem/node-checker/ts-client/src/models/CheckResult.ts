/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */

export type CheckResult = {
    /**
     * Name of the Checker that created the result.
     */
    checker_name: string;
    /**
     * Headline of the result, e.g. "Healthy!" or "Metrics missing!".
     */
    headline: string;
    /**
     * Score out of 100.
     */
    score: number;
    /**
     * Explanation of the result.
     */
    explanation: string;
    /**
     * Links that might help the user fix a potential problem.
     */
    links: Array<string>;
};

