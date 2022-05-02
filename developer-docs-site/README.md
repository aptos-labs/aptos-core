# Developer Documentation

   - [Installation](#installation)   
      - [Requirements](#requirements)   
   - [Fork and clone the Aptos repo](#fork-and-clone-the-aptos-repo)   
   - [Build and serve the docs locally](#build-and-serve-the-docs-locally)   
   - [Build static html files](#build-static-html-files)   

This Aptos Developer Documenatation is built using [Docusaurus 2](https://docusaurus.io/). Follow the below steps to build the docs locally to test your contribution.

## Installation

**IMPORTANT**: These installation steps apply to macOS environment.

### Requirements

Before you proceed, make sure you install the following tools.

- Install [Node.js](https://nodejs.org/en/download/) by executing the below command on your Terminal:

```
brew install node
```

- Install the latest [Yarn](https://classic.yarnpkg.com/lang/en/docs/install/#mac-stable) by executing the below command on your Terminal:

```
brew install yarn
```

## Fork and clone the Aptos repo

>**NOTE**: See this [GitHub documentation for how to configure for a fork](https://docs.github.com/en/pull-requests/collaborating-with-pull-requests/working-with-forks/syncing-a-fork).

1. Fork the Aptos Core repo by clicking on the **Fork** on the top right of this repo page:
https://github.com/aptos-labs/aptos-core

2. Clone your fork.

  ```
  git clone https://github.com/<YOUR-GITHUB-USERID>/aptos-core

  ```

## Build and serve the docs locally

1. `cd` into the `developer-docs-site` directory in your clone.

  ```
  cd aptos-core/developer-docs-site
  ```
2. Run `yarn`.

  ```
  yarn
  ```
This step will configure the Docusaurus static site generator.

3. Start the Yarn server locally. This will also open the locally built docs in your default browser.

> **NOTE**: This step will not generate static html files, but will render the docs dynamically.

  ```
  yarn start
  ```

## Build static html files

Execute the below steps if you want to generate static html documentation files. A `build` directory will be created with the static html files and assets contained in it.

1. Make sure you install Yarn dependencies.

  ```
  yarn install
  ```
2. Build static html files with Yarn.

  ```
  $ yarn build
  ```

This command generates static html content and places it in the `build` directory.

3. Finally, use the below command to start the documentation server on your localhost.

  ```
  npm run serve
  ```
