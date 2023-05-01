# Developer Documentation

   - [Installation](#installation)
      - [Requirements](#requirements)
   - [Fork and clone the Aptos repo](#fork-and-clone-the-aptos-repo)
   - [Build and serve the docs locally](#build-and-serve-the-docs-locally)
   - [Build static html files](#build-static-html-files)
   - [Debug/Format files](#debugging)

This Aptos Developer Documentation is built using [Docusaurus 2](https://docusaurus.io/) and displayed on https://aptos.dev/. Follow the below steps to build the docs locally and test your contribution.

We now use [lychee-broken-link-checker](https://github.com/marketplace/actions/lychee-broken-link-checker) to check for broken links in the GitHub Markdown. We are a corresponding link checker for pages on Aptos.dev.

With results visible at:
https://github.com//aptos-labs/aptos-core/actions/workflows/links.yml


## Installation

**IMPORTANT**: These installation steps apply to macOS environment.

### Requirements

Before you proceed, make sure you install the following tools.

- Install [Node.js](https://nodejs.org/en/download/) by executing the below command on your Terminal:

```
brew install node
```

- Install the latest [pnpm](https://pnpm.io/installation) by executing the below command on your Terminal:

```
curl -fsSL https://get.pnpm.io/install.sh | sh -
```

## Clone the Aptos repo

  ```
  git clone https://github.com/aptos-labs/aptos-core.git

  ```

## Build and serve the docs locally

1. `cd` into the `developer-docs-site` directory in your clone.

  ```
  cd aptos-core/developer-docs-site
  ```
2. Run `pnpm`.

  ```
  pnpm install
  ```
This step will configure the Docusaurus static site generator.

3. Start the server locally. This will also open the locally built docs in your default browser.

> **NOTE**: This step will not generate static html files, but will render the docs dynamically.

  ```
  pnpm start
  ```

  4. See your changes staged at: http://localhost:3000/

  5. Create a pull request with your changes as described in our [Contributing](https://github.com/aptos-labs/aptos-core/blob/main/CONTRIBUTING.md) README.

## (Optional) Build static html files

Execute the below steps if you want to generate static html documentation files. A `build` directory will be created with the static html files and assets contained in it.

1. Make sure you install dependencies.

  ```
  pnpm install
  ```
2. Build static html files with pnpm.

  ```
  pnpm build
  ```

This command generates static html content and places it in the `build` directory.

3. Finally, use the below command to start the documentation server on your localhost.

  ```
  pnpm run serve
  ```

## Debugging

Fix formatting issues by running:

```
pnpm fmt
```

## Regenerating contributors
The src/contributors.json file (which powers the list of Authors at the bottom of doc pages) needs to be manually generated.

In order to generate the contributor map you must authenticate with GitHub. The best way to do that is using GitHub CLI ([installation guide(https://github.com/cli/cli#installation)]). Once you have the GitHub CLI installed, you can run the following command to authenticate:
```
gh auth login --scopes read:user,user:email
```

Once that is done, you can generate the map with this command:
```
pnpm contributors
```
