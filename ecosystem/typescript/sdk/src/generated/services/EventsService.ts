/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
import type { Address } from '../models/Address';
import type { IdentifierWrapper } from '../models/IdentifierWrapper';
import type { MoveStructTag } from '../models/MoveStructTag';
import type { U64 } from '../models/U64';
import type { VersionedEvent } from '../models/VersionedEvent';

import type { CancelablePromise } from '../core/CancelablePromise';
import type { BaseHttpRequest } from '../core/BaseHttpRequest';

export class EventsService {

    constructor(public readonly httpRequest: BaseHttpRequest) {}

    /**
     * Get events by creation number
     * Event streams are globally identifiable by an account `address` and
     * monotonically increasing `creation_number`, one per event stream
     * originating from the given account. This API returns events
     * corresponding to that event stream.
     * @param address Address of account with or without a `0x` prefix. This is should be
     * the account that published the Move module that defined the event
     * stream you are trying to read, not any account he event might be
     * affecting.
     * @param creationNumber Creation number corresponding to the event stream originating
     * from the given account.
     * @param start Starting sequence number of events.
     *
     * By default, will retrieve the most recent events
     * @param limit Max number of events to retrieve.
     *
     * Mo value defaults to default page size
     * @returns VersionedEvent
     * @throws ApiError
     */
    public getEventsByCreationNumber(
        address: Address,
        creationNumber: U64,
        start?: U64,
        limit?: number,
    ): CancelablePromise<Array<VersionedEvent>> {
        return this.httpRequest.request({
            method: 'GET',
            url: '/accounts/{address}/events/{creation_number}',
            path: {
                'address': address,
                'creation_number': creationNumber,
            },
            query: {
                'start': start,
                'limit': limit,
            },
        });
    }

    /**
     * Get events by event handle
     * This API uses the given account `address`, `event_handle`, and
     * `field_name` to build a key that globally identify an event stream.
     * It then uses this key to return events from tha stream.
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
