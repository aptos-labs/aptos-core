---
title: "Aptos Labs Developer Portal"
---

import BetaNotice from '../../src/components/_dev_portal_beta_notice.mdx';

<BetaNotice />

The [Aptos Labs Developer Portal](https://developers.aptoslabs.com) is a your gateway to access Aptos Labs provided APIs in a quick and easy fashion to power your dApp.
It consists of a Portal (UI) and a set of API Gateways operated by Aptos Labs.

The Developer Portal aims to make it easier to build dApps by:

1. Providing [unified domain names/URLs](../nodes/networks.md) for each API.
2. Giving you personalized examples on how to use each API.
3. Observability into your personal usage, error rates and latency of APIs.
4. Rate limiting by API developer account/app instead of origin IP.
5. (Coming Soon) Customizable Rate limits for high traffic apps.

In order to create an Aptos Labs developer account simply go to https://developers.aptoslabs.com/ and follow the instructions.

### Default Rate Limits for Developer Portal accounts

Currently the following rate limits apply:

1. GRPC Transaction Stream: 20 concurrent streams per user
2. Fullnode API: 5000 requests per 5 minutes sliding window.
3. GraphQL API: 5000 requests per 5 minutes sliding window.

   Note that requests for the Fullnode API / GraphQL API are counted separately, so you can make 5000 Fullnode API requests AND 5000 GraphQL API requests in the same 5 minutes window. The rate limit is applied as a continuous sliding window.

Rate limits are customizable per user upon request. If you have a use-case that requires higher rate limits than the default, please open a support case through one of the supported channels in the portal.

### Known Limitations

1. Only authenticated access supported.

   At the moment the new URLs introduced by the Developer Portal / API Gateway only support requests with an API Key (Bearer authentication).
   Effectively this means you can only use the new API gateway provided URLs from backend apps that can securely hold credentials.
   We plan to add soon support for anonymous authentication in combination with more sophisticated rate limit protections, which then makes then these new URLs usable in end-user / client-side only apps like Browser Wallets etc.
