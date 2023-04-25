# Aptos SDK Specification Document

[Table of Content](#table-of-content) | [Templates](./templates)

![Status](https://img.shields.io/badge/version-1.0-brightgreen.svg)

## Overview

The goal of this document is to set a shared standard for implementation and development of all Aptos SDKs.

This document is a work in progress, and should be changed and updated as changes are made when developer needs are discovered.

### Requirement Prioritization

The following document follows the [MoSCoW](https://en.wikipedia.org/wiki/MoSCoW_method) method of prioritising rules. Please follow the following guidelines when evaluating rules.

- `MUST` - Rules labeled as **must** are requirements that should not be deviated from at any cost
- `SHOULD` - Rules labeled as **should** are requirements that could be deviated from if needed, though this will have to be documented and cleared with all stakeholders before it can be disregarded.
- `COULD` - Rules labeled as **could** are requirements that are desirable but not necessary and therefore would be nice to have where time and resources permit.

We do not use the fourth **`won't`** level in this specification.

## Table of Contents

- Maintenance Requirements
  - [1. Source Control](#1-source-control)
  - [2. Releases & Versioning](#2-releases--versioning)
  - [3. CI Server](#3-ci-server)
- Additional Content Requirements
  - [4. Documentation](#4-documentation)
  - [5. Testing](#5-testing)
  - [6. Linting](#6-linting)
- Dependencies & Infrastructure Requirements
  - [7. Dependencies](#7-dependencies)
  - [8. HTTP Client](#8-http-client)
  - [9. Logging](#9-logging)
  - [10. Reporting](#10-reporting)
- Initialization & Interaction Requirements
  - [11. Initialization](#11-initalization)
  - [12. Namespacing](#12-namespacing)
  - [13. Method Syntax](#13-method-syntax)
  - [14. Error Handling](#14-error-handling)
- API Mapping Requirements
  - [15. API Calls](#15-api-calls)
  - [16. Responses](#16-responses)
  - [17. Requests](#17-requests)
  - [18. Pagination](#18-pagination)
- Key Developer Experience Interactions
  - [19. Successful Path Interactions](#19-successful-path-interactions)
  - [20. Unsuccessful Path Interactions](#20-unsuccessful-path-interactions)
- Specific Language Requirements
  - [21. Ruby](#21-ruby)
  - [22. Node / Javascript](#22-node--javascript)
  - [23. Python](#23-python)
  - [24. Java](#24-java)

## Maintenance Requirements

### 1. Source Control

- [ ] **1.1** The source code for the SDK **must** be maintained within Git version control
- [ ] **1.2** The source code **must** be hosted publicly
- [ ] **1.3** Development of new features **should** happen on feature branches
- [ ] **1.4** Feature branches **should** pass all tests and linting before they can be merged into the `main` branch
- [ ] **1.5** Source control **should** contain tags for each release of the SDK
- [ ] **1.6** The `main` branch **should** be kept in a condition that allows for direct use through checkout
- [ ] **1.7** The source code **should** use GitHub for public hosting

### 2. Releases & Versioning

- [ ] **2.1** The SDK **must** use [Semantic Versioning](http://semver.org/) to increment the version number as changes are made
- [ ] **2.2** For every new release the `CHANGELOG` file **must** to be updated with the `Major`, `Minor` and `Patch` changes
- [ ] **2.3** A release package **must** include the documentation `README` file
- [ ] **2.4** A release package **must** include the `LICENSE` file
- [ ] **2.5** A release package **must** include the `CHANGELOG` file

- [ ] **2.6** The version number of the SDK **should** be independent of the API version
- [ ] **2.7** A release package **should** not include unnecessary source code files or intermediarry files for the SDK.

- [ ] **2.8** The name of the SDK **should** follow language best practices, and be one of `aptos` or `Aptos`
- [ ] **2.9** If the preferred name of the SDK is not available, it **could** be one of `aptos-sdk`, `AptosSDK`, or `aptosdev`.
- [ ] **2.10** As soon as the first public version of the library has been signed off, the version **should** be bumped to `1.0.0`
- [ ] **2.11** The version number of the SDK **could** be incremented when the SDK has gathered enough changes to warrant a new release
- [ ] **2.12** New releases **could** be deployed automatically to the package manager using the CI server

### 3. CI Server

- [ ] **3.1** A Continuous Integration (CI) server **must** be used to automatically test any branch of the Git repository
- [ ] **3.3** The CI server **must** test against all current LTS language versions
- [ ] **3.4** The CI server **could** test against popular non-LTS versions
- [ ] **3.5** The CI server **could** test on different platforms, including Windows, Linux, and macOS.
- [ ] **3.6** The CI server **could** test new Git tags, and build and push the package to the package manager

## Additional Content Requirements

### 4. Documentation

- [ ] **4.1** The SDKs **must** include a `README` file
  - [ ] **4.1.4** The `README` file **should** be written in Markdown
  - [ ] **4.1.5** The `README` file **must** have instructions on how to install the SDK using a package manager
  - [ ] **4.1.1** The `README` file **should** include a version badge
  - [ ] **4.1.2** The `README` file **should** include a test status badge
  - [ ] **4.1.3** The `README` file **must** link to the `LICENSE` file
  - [ ] **4.1.6** The `README` file **could** have instructions on how to install the SDK from version control
  - [ ] **4.1.8** The `README` file **should** document all the different ways the SDK can be initialized
  - [ ] **4.1.12** The `README` file **must** document any installation requirements and prerequisites
  - [ ] **4.1.13** The `README` file **should** to official support channels
- [ ] **4.4** The GitHub repository **should** have a title in the format "Typescript library for the Aptos network"
- [ ] **4.5** The GitHub repository **should** have the following tags: `aptos`, `blockchain`, `web3`, `sdk`, `library`
- [ ] **4.6** The SDKs **must** include a `CHANGELOG` file
- [ ] **4.7** The SDKs **must** include a `CODE_OF_CONDUCT` file
- [ ] **4.8** The SDKs **must** include a `CONTRIBUTING` file
  - [ ] **4.8.1** The Contribution Guidelines **should** include instructions on how to run the SDK in development/testing mode.
- [ ] **4.9** The SDKs **must** include a `ISSUE_TEMPLATE` file
- [ ] **4.10** The SDKs **must** include a `PULL_REQUEST_TEMPLATE` file
- [ ] **4.11** The SDKs **must** include a `SUPPORT` file

> Templates for a lot of these files have been provided in the [templates](./templates) folder

### 5. Testing

- [ ] **5.1** The SDKs **must** be thoroughly tested
- [ ] **5.2** The tests **should** have integration tests to make the network calls
- [ ] **5.3** The tests **should** test responses
- [ ] **5.4** For any real API calls, the tests **must** use the Aptos tesnet or devnet networks

### 6. Linting

- [ ] **6.1** The SDKs **must** have their files linted
- [ ] **6.2** The linting **must** ensure that tabs/spaces are consistently used
- [ ] **6.3** The linting **should** ensure no trailing whitespace is left in the code
- [ ] **6.4** The linting **should** ensure quotes and brackets are consistently applied
- [ ] **6.5** The linting **could** ensure semicolons are present when needed
- [ ] **6.6** The linting **could** ensure comments are present on public methods

## Dependencies & Infrastructure Requirements

### 7. Dependencies

- [ ] **7.1** The SDK **must** limit its runtime dependencies
- [ ] **7.2** The SDK **should** have no runtime dependencies
- [ ] **7.3** The SDK **could** use any amount of development and test dependencies

### 8. HTTP Client

- [ ] **8.1** The SDK **must** use a well supported HTTP client
- [ ] **8.1** The SDK **should** use a HTTP2 supported client
- [ ] **8.2** A HTTP client from the standard libraries **should** be used
- [ ] **8.4** The SDK **could** allow a developer to provide an alternative HTTP client

<!-- ### 9. Logging

- [ ] **9.1** The SDK **must** be able to log activities to a logger
- [ ] **9.2** The logger **should** use the default runtime log
- [ ] **9.3** The logger **must** allow enabling/disabling of debug mode per instance
- [ ] **9.4** The logger **should** allow a developer to provide an alternative logger
- [ ] **9.5** When debugging is enabled, the logger **should** log (and only log) the request object, response object, and optionally any raw HTTP response object of no response object could be formed. -->

### 10. Reporting

- [ ] **10.1** The SDK **must** identify requests to the API as originating from the SDK
- [ ] **10.2** The SDK **must** report the SDK version number to the API
- [ ] **10.5** The SDK **should** pass a custom header `x-aptos-client` with the format `<sdk-id>/<sdk-version>`
  - Example with known sdk version: `aptos-ts-sdk/1.8.4`

## Initialization & Interaction Requirements

### 11. Initialization

- [ ] **11.11** The SDK client **must** allow selection of the base URL by name (`devnet` and `tesnet` and `mainnet`)
- [ ] **11.12** The SDK client **must** allow for setting a custom base URL directly
- [ ] **11.10** The SDK client **could** accept an alternative HTTP client

### 13. Method Syntax

- [ ] **13.2** The SDK API calls **must** allow for a method to take an account adress to fetch a resources
  - Example: `getResources(0x123)` : `GET /v1/resources/0x123`
  - Example: `getModules(0x123)` : `GET /v1/modules/0x123`
- [ ] **13.3** When making a transaction request, the SDK API calls **should** accept a json payload data or a bcs serialized payload data to be submitted
- [ ] **13.6** The SDK API calls **should** allow for a method to take a Hex string account adress
- [ ] **13.4** In asynchronous programming languages, the SDK API calls **should** use async/await syntax or a promise to be returned
  - Example with await `await getResources(0x123)`
  - Example with promise `getResources(0x123).then(...).catch(...)`
- [ ] **13.5** The SDK API calls **should** return a response object

### 14. Error Handling

- [ ] **14.1** The SDK **should** raise a `ApiError` for any request that did not result a HTTP 200 or 201 response code
- [ ] **14.4** The response error object **should** contain a `message` attribute containing the message of the error, e.g. `account_not_found`.
- [ ] **14.5** The response error object **should** contain a `error_code` attribute containing the error code, e.g. `404`.
- [ ] **14.5** The response error object **should** contain a `vm_error_code` attribute containing the vm error code, e.g. `0`.

## API Mapping Requirements

### 15. API Calls

- [ ] **15.1** The SDK **should** define the `reference-data` namespace
  - [ ] **15.1.1** The SDK **must** implement the [`GET /v2/reference_data/urls/checkin-links`](https://mobile-services.amadeus.com/catalogue/31#learn) endpoint
    - Example: `amadeus.reference_data.urls.checkin_links.get({ airline: 'BA' })`
  - [ ] **15.1.2** The SDK **must** implement the [`GET /v2/reference_data/locations`](https://mobile-services.amadeus.com/catalogue/32#learn) endpoint
    - Example: `amadeus.reference_data.locations.get({ keyword: 'LON' })`
  - [ ] **15.1.3** The SDK **must** implement the [`GET /v2/reference_data/locations/airports`](https://mobile-services.amadeus.com/catalogue/33#learn) endpoint
    - Example: `amadeus.reference_data.locations.airports.get({ latitude: '0.0', longitude: '0.0' })`
- [ ] **15.2** The SDK **should** define the `shopping` namespace
  - [ ] **15.2.1** The SDK **must** implement the [`GET /v1/shopping/flight-destinations`](https://mobile-services.amadeus.com/catalogue/35#learn) endpoint
    - Example: `amadeus.shopping.flight_destinations.get({ origin: 'LAX' })`
  - [ ] **15.2.2** The SDK **must** implement the [`GET /v1/shopping/flight-offers`](https://mobile-services.amadeus.com/catalogue/36#learn) endpoint
    - Example: `amadeus.shopping.flight_offers.get({ origin: 'LAX', destination: 'LHR', departureDate: '2020-12-01' })`
  - [ ] **15.2.3** The SDK **must** implement the [`GET /v1/shopping/flight-dates`](https://mobile-services.amadeus.com/catalogue/37#learn) endpoint
    - Example: `amadeus.shopping.flight_dates.get({ origin: 'LAX', destination: 'LHR' })`
- [ ] **15.3** The SDK **should** define the `travel/analytics` namespace
  - [ ] **15.3.1** The SDK **must** implement the [`GET /v1/travel/analytics/fare-searches`](https://mobile-services.amadeus.com/catalogue/30#learn) endpoint
    - Example: `amadeus.travel.analytics.fare_searches.get({ origin: 'LAX', sourceCountry: 'US', period: 2015 })`
  - [ ] **15.3.2** The SDK **must** implement the [`GET /v1/travel/analytics/air-traffics`](#) endpoint
    - Example: `amadeus.travel.analytics.air_traffics.get({ origin: 'LAX', period: '2015-07' })`
- [ ] **15.4** The SDK **should** define the `shopping/hotels` namespace
  - [ ] **15.4.1** The SDK **must** implement the [`GET /v1/shopping/hotel-offers`](#) endpoint
    - Example: `amadeus.shopping.hotel_offers.get({ cityCode: 'LAX' }`
  - [ ] **15.4.2** The SDK **must** implement the [`GET /v1/shopping/hotels/:hotel_id/hotel-offers`](#) endpoint
    - Example: `amadeus.shopping.hotels.get(123).hotel_offers.get({ checkInDate: '2018-12-01', checkOutDate: '2018-12-03' })`
  - [ ] **15.4.3** The SDK **must** implement the [`GET /v1/shopping/hotels/:hotel_id/offers/:offer_id`](#) endpoint
    - Example: `amadeus.shopping.hotels.get(123).hotel_offers.get(345)`

### 16. Responses

- [ ] **16.1** The SDK **must** return a response object where possible, instead of the JSON data directly
- [ ] **16.2** The response object **must** contain a `result` attribute with the parsed JSON content if the content could be parsed
- [ ] **16.4** The response object **must** contain a `data` attribute with the content from the data key from the `result` hash, if it is present
- [ ] **16.5** The response object **must** be parsed as well when an error occurred
- [ ] **16.6** An error **must** be thrown if the response was JSON but could not be parsed
- [ ] **16.7** The response object **must** contain a `statusCode` attribute with HTTP status code of the response
- [ ] **16.8** The response object **must** contain a `request` attribute with the details of the original request made
- [ ] **16.9** The response object **should** contain a `parsed` attribute which should be `true` when the JSON was successfully parsed
- [ ] **16.10** The response object **should** be able to deal with any new parameters returned from the API without needing an SDK update. In other words, the class definition of response objects should not define the attributes of the object statically
- [ ] **16.11** The response object **should** remain lightweight and not contain any instance variables besides those needed to track what the API returned, keeping the logged output of the instance to the important details of the API call.

### 17. Requests

- [ ] **17.1** The SDK **must** keep track of the details of a request in a request object that's accessible through the response object
- [ ] **17.2** The request object **must** track the host, port, verb, path, params, bearerToken, headers (including User Agent) used for the call
- [ ] **17.3** The request object **should** remain lightweight and not contain any instance variables besides those needed to track what API call was made, keeping the logged output of the instance to the important details of the API call.

### 18. Pagination

- [ ] **18.1** The SDK **must** allow for easy pagination of responses
- [ ] **18.2** The SDK **must** expose `.next`, `.first`, `.last`, `.previous` methods on the API client to find the relevant page for a response
  - Example given a previous response: `let next_response = amadeus.next(response);`
- [ ] **18.3** The SDK **should** not expose any pagination function on the response objects, as this would require the response objects to keep a reference to the API client, limiting the ability to easily debug a response object

## Key Developer Experience Interactions

### 19. Successful Path Interactions

- [ ] **19.1** Finding a location by type **should** allow for using a built in type
  - Ruby example: `locations = amadeus.reference_data.locations.get(keyword: 'lon', subType: Amadeus::Location::Airport)`
  - Node example: `let locations = amadeus.referenceData.locations.get({ keyword: 'lon', subType: Amadeus.location.airport });`
- [ ] **19.2** Making a query using a location code **should** be able to use the response from a location query
  - Ruby example: `amadeus.foo.bar(origin: locations.data.first['iataCode'])`
  - Node example: `amadeus.foo.bar({ origin: locations.data.first.iataCode });`

### 20. Unsuccessful Path Interactions

- [ ] **20.1** When incorrect credentials are provided, the error returned **should** be clear even when debug mode is off
<details>
<summary>Ruby example:</summary>

```ruby
amadeus.get('/foo/bar')
```

```ruby
W, [2018-02-19T16:06:29.881202 #67814]  WARN -- Amadeus AuthenticationError: {
  "error": "invalid_client",
  "error_description": "Client credentials are invalid",
  "code": 38187,
  "title": "Invalid parameters"
}
```

</details>

<details>
<summary>Node example:</summary>
   
```js
amadeus.client.get('/foo/bar').then(...).catch(...);
```

```js
Amadeus AuthenticationError { error: 'invalid_client',
  error_description: 'Client credentials are invalid',
  code: 38187,
  title: 'Invalid parameters' }
```

</details>

<details>
<summary>Python example:</summary>
   
```js
amadeus.get('/foo/bar')
```

```js
Amadeus AuthenticationError: {'code': 38187,
 'error': 'invalid_client',
 'error_description': 'Client credentials are invalid',
 'title': 'Invalid parameters'}
```

</details><br/>

- [ ] **20.2** When an unknown path is provided, the error returned **should** be clear even when debug mode is off
<details>
<summary>Ruby example:</summary>

```ruby
amadeus.get('/foo/bar')
```

```ruby
W, [2018-02-19T16:06:13.523516 #67786]  WARN -- Amadeus NotFoundError: [
  {
    "code": 38196,
    "title": "Resource not found",
    "detail": "The targeted resource doesn't exist",
    "status": 404
  }
]
```

</details>

<details>
<summary>Node example:</summary>
   
```js
amadeus.client.get('/foo/bar').then(...).catch(...);
```

```js
Amadeus NotFoundError [ { code: 38196,
    title: 'Resource not found',
    detail: 'The targeted resource doesn\'t exist',
    status: 404 } ]
```

</details>

<details>
<summary>Python example:</summary>
   
```python
amadeus.get('/foo/bar')
```

```python
Amadeus NotFoundError: [{'code': 38196,
  'detail': "The targeted resource doesn't exist",
  'status': 404,
  'title': 'Resource not found'}]
```

</details><br/>

- [ ] **20.3** When incorrect params are provided, the error returned **should** be clear even when debug mode is off
<details>
<summary>Ruby example:</summary>

```ruby
amadeus.reference_data.locations.get(
  subType: Amadeus::Location::ANY
)
```

```js
W, [2018-02-19T16:05:55.923870 #67772]  WARN -- Amadeus ClientError: [
  {
    "status": 400,
    "code": 32171,
    "title": "MANDATORY DATA MISSING",
    "detail": "Missing mandatory query parameter",
    "source": {
      "parameter": "keyword"
    }
  }
]
```

</details>

<details>
<summary>Node example:</summary>
   
```js
amadeus.referenceData.locations.get({
  keyword: 'lon'
}).then(...).catch(...);
```

```js
Amadeus ClientError [ { status: 400,
    code: 32171,
    title: 'MANDATORY DATA MISSING',
    detail: 'Missing mandatory query parameter',
    source: { parameter: 'subType' } } ]
```

</details>

<details>
<summary>Ruby example:</summary>
   
```python
amadeus.reference_data.locations.get(
  subType: Amadeus::Location::ANY
)
```

```python
Amadeus ClientError: [{'code': 32171,
  'detail': 'Missing mandatory query parameter',
  'source': {'parameter': 'keyword'},
  'status': 400,
  'title': 'MANDATORY DATA MISSING'}]
```

</details><br/>

- [ ] **20.4** When a server error occurs, the error returned **should** be clear even when debug mode is off

<details>
<summary>Ruby example:</summary>
   
```js
amadeus.get('/something/that/errors/');
```

```js
W, [2018-02-19T16:07:42.651272 #67846]  WARN -- Amadeus ServerError: [
  {
    "code": 38189,
    "title": "Internal error",
    "detail": "An internal error occured, please contact your administrator",
    "status": 500
  }
]
```

</details>

<details>
<summary>Node example:</summary>
   
```js
amadeus.get('/something/that/errors/').then(...).catch(...);
```

```js
Amadeus ServerError [ { code: 38189,
    title: 'Internal error',
    detail: 'An internal error occured, please contact your administrator',
    status: 500 } ]
```

</details>

<details>
<summary>Python example:</summary>
   
```python
amadeus.get('/something/that/errors/');
```

```pyton
Amadeus ClientError: [{'code': 38189,
    "code": 38189,
    "title": "Internal error",
    "detail": "An internal error occured, please contact your administrator",
    "status": 500 }]
```

</details><br/>

- [ ] **20.5** When a network error occurs, the error returned **should** be clear even when debug mode is off

<details>
<summary>Ruby example:</summary>
   
```ruby
amadeus.get('/something/that/errors/');
```

```ruby
W, [2018-02-19T16:13:14.374444 #68060]  WARN -- Amadeus NetworkError: nil
```

</details>

<details>
<summary>Node example:</summary>
   
```js
amadeus.get('/something/that/errors/').then(...).catch(...);
```

```js
Amadeus NetworkError null
```

</details>

<details>
<summary>Python example:</summary>
   
```python
amadeus.get('/something/that/errors/');
```

```python
Amadeus NetworkError: None
```

</details><br/>

- [ ] **20.6** When a rate limit occurs, the error returned **should** be clear even when debug mode is off

<details>
<summary>Ruby example:</summary>
   
```ruby
amadeus.get('/something/that/rate/limits/');
```

```ruby
W, [2018-02-19T16:07:42.651272 #67846]  WARN -- Amadeus ServerError: [
  {
    code: 38194,
    title: 'Too many requests',
    detail: 'The network rate limit is exceeded, please try again later',
    status: 429
  }
]
```

</details>

<details>
<summary>Node example:</summary>
   
```js
amadeus.get('/something/that/errors/').then(...).catch(...);
```

```js
Amadeus ClientError [ { code: 38194,
    title: 'Too many requests',
    detail: 'The network rate limit is exceeded, please try again later',
    status: 429 } ]
```

</details>

<details>
<summary>Python example:</summary>
   
```python
amadeus.get('/something/that/rate/limits/');
```

```python
Amadeus ClientError: [{'code': 38194,
  'detail': 'The network rate limit is exceeded, please try again later',
  'status': 429,
  'title': 'Too many requests'}]
```

</details>
<br/>

## Specific Language Requirements

### 21. Ruby

- [ ] **21.1** The SDK **must** support Ruby 2.2+
- [ ] **21.2** The SDK **should** support JRuby

### 22. Node / Javascript

- [ ] **22.1** The SDK **should** promises
- [ ] **22.2** The SDK **could** support ES7's `async/await`
- [ ] **22.3** The SDK **should** be written in ES6+
- [ ] **22.4** The SDK **should** work in an ES5 environment
- [ ] **22.5** The SDK **should** support ES6 modules

### 23. Python

- [ ] **23.1** The SDK **should** support Python 2 and 3

### 24. Java

- [ ] **24.1** The SDK **should** support both the regular JRE, and the Android runtime
