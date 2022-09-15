/* eslint-disable no-console */
// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import constate from 'constate';
import {
  useCallback,
  useEffect,
  useMemo,
} from 'react';
import { AnalyticsBrowser } from '@segment/analytics-next';
import { CombinedEventParams, AnalyticsEventTypeValues } from 'core/utils/analytics/events';
import { getBrowser } from 'core/utils/browser';
import { getOS } from 'core/utils/os';
import { defaultNetworkName } from 'shared/types';
import { useLocation } from 'react-router-dom';
import { useAppState } from './useAppState';

const isDevelopment = (!process.env.NODE_ENV || process.env.NODE_ENV === 'development');
const writeKey = process.env.REACT_APP_SEGMENT_WRITE_KEY;

interface AnalyticsGeneralEventParams {
  eventType: AnalyticsEventTypeValues;
  page: string;
  params?: CombinedEventParams;
  screen: string;
  value?: number;
}

type AnalyticsPageEventParams = Omit<AnalyticsGeneralEventParams, 'screen' | 'eventType'>;

type AnalyticsScreenEventParams = Omit<AnalyticsGeneralEventParams, 'page' | 'eventType'>;

type AnalyticsEventParams = Omit<AnalyticsGeneralEventParams, 'page' | 'screen'>;

/**
 * @summary Segment analytics hook that communicates with analytics-node
 */
export const [AnalyticsProvider, useAnalytics] = constate(() => {
  const { activeAccountAddress, activeNetworkName } = useAppState();
  const { pathname } = useLocation();
  const analytics = useMemo(
    () => ((writeKey) ? AnalyticsBrowser.load({
      writeKey,
    }) : undefined),
    [],
  );
  const userId = (activeAccountAddress) || undefined;

  /**
   * @summary Analytics event track page
   * @see https://segment.com/docs/connections/spec/screen/
   * @param page page route that user is going to
   *
   * @example
   * ```ts
   * trackPage({
   *   page: Routes.settings.path,
   * });
   * ```
   */
  const trackPage = useCallback(({
    page,
  }: AnalyticsPageEventParams) => {
    if (isDevelopment || !analytics) {
      return;
    }

    const eventEnv = (isDevelopment) ? 'dev_event' : 'event';

    analytics.user().then((user) => {
      analytics.page(page, {
        properties: {
          $browser: getBrowser({ os: getOS() })?.toString(),
          $os: getOS()?.toString(),
          eventEnv,
          network: activeNetworkName,
          walletId: user.anonymousId()?.toString(),
        },
        timestamp: new Date(),
        userId,
      });
    }).catch((err) => console.error(err));
  }, [analytics, activeNetworkName, userId]);

  /**
   * @summary Analytics event track screen - (different than page,
   *          pages are designated with routes while screens
   *          can be non-route based)
   * @see https://segment.com/docs/connections/spec/screen/
   * @param {String} screen
   *
   * @example
   * ```ts
   * trackScreen({
   *   screen: 'Transfer drawer',
   * });
   * ```
   */
  const trackScreen = useCallback(({
    screen,
  }: AnalyticsScreenEventParams) => {
    if (isDevelopment || !analytics) {
      return;
    }

    const eventEnv = (isDevelopment) ? 'dev_event' : 'event';

    analytics.user().then((user) => {
      analytics.screen(screen, screen, {
        properties: {
          $browser: getBrowser({ os: getOS() })?.toString(),
          $os: getOS()?.toString(),
          eventEnv,
          network: activeNetworkName,
          walletId: user.anonymousId()?.toString(),
        },
        timestamp: new Date(),
        userId,
      });
    }).catch((err) => console.error(err));
  }, [activeNetworkName, analytics, userId]);

  /**
   * @summary Analytics event track event
   * @see https://segment.com/docs/connections/spec/track/
   * @param eventType one of the event types in analyticsEvent
   * @param params optional additional params for an event
   * @param value optional value param for an event
   *
   * @example
   * ```ts
   * trackEvent({
   *   eventType: collectiblesEvents.CREATE_NFT,
   *   params: {
   *     network: NodeNetworkUrl,
   *     ...data,
   *   },
   * });
   * ```
   */
  const trackEvent = useCallback(({
    eventType,
    params,
    value,
  }: AnalyticsEventParams) => {
    if (isDevelopment || !analytics) {
      return;
    }

    const {
      action,
      category,
      label,
    } = eventType;

    const eventEnv = (isDevelopment) ? 'dev_event' : 'event';

    analytics.user().then((user) => {
      analytics.track((label), {
        category,
        name: label,
        properties: {
          ...params,
          $browser: getBrowser({ os: getOS() })?.toString() || 'chrome',
          $os: getOS()?.toString(),
          action,
          eventEnv,
          network: activeNetworkName || defaultNetworkName,
          value,
          walletId: user.anonymousId()?.toString(),
        },
        timestamp: new Date(),
        type: 'track',
        userId,
      });
    }).catch((err) => console.error(err));
  }, [activeNetworkName, analytics, userId]);

  useEffect(() => {
    trackPage({ page: pathname });
  }, [pathname, trackPage]);

  return {
    trackEvent,
    trackPage,
    trackScreen,
  };
});
