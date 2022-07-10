// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import ReactGA from 'react-ga4';
import { CombinedEventParams, AnalyticsEventTypeValues } from './events';

interface GoogleAnalyticsEventParams {
  eventType: AnalyticsEventTypeValues;
  params?: CombinedEventParams;
  value?: number;
}

/**
 * @summary Google Analytics event
 * @param eventType one of the event types in analyticsEvent
 * @param params optional additional params for an event
 * @param value optional value param for an event
 *
 * @example
 * ```ts
 * Analytics.event({
 *   eventType: collectiblesEvents.CREATE_NFT,
 *   params: {
 *     network: aptosNetwork,
 *     ...data,
 *   },
 * });
 * ```
 */
export const googleAnalyticsEvent = ({
  eventType,
  params,
  value,
}: GoogleAnalyticsEventParams) => {
  const {
    action,
    category,
    label,
  } = eventType;
  ReactGA.event({
    action,
    category,
    label,
    nonInteraction: true,
    transport: 'xhr',
    value,
  }, params);
};

export const Analytics = {
  event: googleAnalyticsEvent,
};

export default Analytics;
