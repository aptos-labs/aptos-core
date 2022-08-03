/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
import type { Address } from '../models/Address';
import type { Event } from '../models/Event';
import type { EventKey } from '../models/EventKey';
import type { IdentifierWrapper } from '../models/IdentifierWrapper';
import type { MoveStructTagParam } from '../models/MoveStructTagParam';
import type { U64 } from '../models/U64';

import type { CancelablePromise } from '../core/CancelablePromise';
import type { BaseHttpRequest } from '../core/BaseHttpRequest';

export class EventsService {

    constructor(public readonly httpRequest: BaseHttpRequest) {}

    /**
     * Get events by event key
     * This endpoint allows you to get a list of events of a specific type
     * as identified by its event key, which is a globally unique ID.
     * @param eventKey
     * @param start
     * @param limit
     * @returns Event
     * @throws ApiError
     */
    public getEventsByEventKey(
        eventKey: EventKey,
        start?: U64,
        limit?: number,
    ): CancelablePromise<Array<Event>> {
        return this.httpRequest.request({
            method: 'GET',
            url: '/events/{event_key}',
            path: {
                'event_key': eventKey,
            },
            query: {
                'start': start,
                'limit': limit,
            },
        });
    }

    /**
     * Get events by event handle
     * This API extracts event key from the account resource identified
     * by the `event_handle_struct` and `field_name`, then returns
     * events identified by the event key.
     * @param address
     * @param eventHandle
     * @param fieldName
     * @param start
     * @param limit
     * @returns Event
     * @throws ApiError
     */
    public getEventsByEventHandle(
        address: Address,
        eventHandle: MoveStructTagParam,
        fieldName: IdentifierWrapper,
        start?: U64,
        limit?: number,
    ): CancelablePromise<Array<Event>> {
        return this.httpRequest.request({
            method: 'GET',
            url: '/accounts/{address}/events/{event_handle}/{field_name}',
            path: {
                'address': address,
                'event_handle': eventHandle,
                'field_name': fieldName,
            },
            query: {
                'start': start,
                'limit': limit,
            },
        });
    }

}
