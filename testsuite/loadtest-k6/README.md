# Typescript k6 load test for Aptos services

See https://github.com/grafana/k6-template-typescript

## Prerequisites

- [k6](https://k6.io/docs/getting-started/installation)
- [NodeJS](https://nodejs.org/en/download/)

## Installation

**Install dependencies**

```bash
pnpm install
```

## Running the test

To run a test written in TypeScript, we first have to transpile the TypeScript code into JavaScript and bundle the project

```bash
pnpm start
```

This command creates the final test files to the `./dist` folder.

Once that is done, we can run our script the same way we usually do, for instance:

```bash
k6 run dist/${YOUR_TEST}-test.js
```
