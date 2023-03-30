/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */

import type { CheckResult } from './CheckResult';

export type CheckSummary = {
    /**
     * Results from all the Checkers NHC ran.
     */
    check_results: Array<CheckResult>;
    /**
     * An aggeregated summary (method TBA).
     */
    summary_score: number;
    /**
     * An overall explanation of the results.
     */
    summary_explanation: string;
};

