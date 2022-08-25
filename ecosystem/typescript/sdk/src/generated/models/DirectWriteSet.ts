/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */

import type { Event } from './Event.js';
import type { WriteSetChange } from './WriteSetChange.js';

export type DirectWriteSet = {
    changes: Array<WriteSetChange>;
    events: Array<Event>;
};

