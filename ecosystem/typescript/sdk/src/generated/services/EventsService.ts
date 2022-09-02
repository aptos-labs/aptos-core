/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
import type { Address } from '../models/Address';
import type { EventKey } from '../models/EventKey';
import type { IdentifierWrapper } from '../models/IdentifierWrapper';
import type { MoveStructTag } from '../models/MoveStructTag';
import type { U64 } from '../models/U64';
import type { VersionedEvent } from '../models/VersionedEvent';

import type { CancelablePromise } from '../core/CancelablePromise';
import type { BaseHttpRequest } from '../core/BaseHttpRequest';

export class EventsService {

    constructor(public readonly httpRequest: BaseHttpRequest) {}

    /**
     * Get events by event key
     * This endpoint allows you to get a list of events of a specific type
     * as identified by its event key, which is a globally unique ID.
     * @param eventKey Event key to retrieve events by
     * @param start Starting sequence number of events.
     *
     * By default, will retrieve the most recent events
     * @param limit Max number of events to retrieve.
     *
     * Mo value defaults to default page size
     * @returns VersionedEvent
     * @throws ApiError
     */
    public getEventsByEventKey(
        eventKey: EventKey,
        start?: U64,
        limit?: number,
    ): CancelablePromise<Array<VersionedEvent>> {
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
     * @param address Address of account with or without a `0x` prefix
     * @param eventHandle Name of struct to lookup event handle e.g. `0x1::account::Account`
     * @param fieldName Name of field to lookup event handle e.g. `withdraw_events`
     * @param start Starting sequence number of events.
     *
     * By default, will retrieve the most recent events
     * @param limit Max number of events to retrieve.
     *
     * Mo value defaults to default page size
     * @returns VersionedEvent
     * @throws ApiError
     */
    public getEventsByEventHandle(
        address: Address,
        eventHandle: MoveStructTag,
        fieldName: IdentifierWrapper,
        start?: U64,
        limit?: number,
    ): CancelablePromise<Array<VersionedEvent>> {
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
