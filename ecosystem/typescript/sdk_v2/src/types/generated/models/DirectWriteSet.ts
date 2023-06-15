/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */

import type { Event } from './Event';
import type { WriteSetChange } from './WriteSetChange';

export type DirectWriteSet = {
    changes: Array<WriteSetChange>;
    events: Array<Event>;
};

