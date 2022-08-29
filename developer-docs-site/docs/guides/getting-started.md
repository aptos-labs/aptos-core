---
title: "Getting Started"
slug: "getting-started"
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';
import ThemedImage from '@theme/ThemedImage';
import useBaseUrl from '@docusaurus/useBaseUrl';

# Getting Started (Draft)

To kick-start your journey as a developer in the Aptos ecosystem, set up your development environment as described in this section.

<ThemedImage
  alt="Development Flow with Aptos CLI"
  sources={{
    light: useBaseUrl('/img/docs/dev-with-aptos-cli.svg'),
    dark: useBaseUrl('/img/docs/dev-with-aptos-cli-dark.svg'),
  }}
/>

### Clone the Aptos-core repo

Start by cloning the `aptos-core` GitHub repo from [GitHub](https://github.com/aptos-labs/aptos-core).

1. Clone the Aptos repo.

      ```
      git clone https://github.com/aptos-labs/aptos-core.git
      ```

2. `cd` into `aptos-core` directory.

    ```
    cd aptos-core
    ```

3. Run the `scripts/dev_setup.sh` Bash script as shown below. This will prepare your developer environment.

    ```
    ./scripts/dev_setup.sh
    ```

4. Update your current shell environment.

    ```
    source ~/.cargo/env
    ```
5. Skip this step if you are not installing an Aptos node.

    <Tabs>
    <TabItem value="devnet" label="Devnet" default>

    Checkout the `devnet` branch using:

    ```
    git checkout --track origin/devnet
    ```
    </TabItem>
    <TabItem value="testnet" label="Testnet" default>

    Checkout the `testnet` branch using:

    ```
    git checkout --track origin/testnet
    ```
    </TabItem>
    </Tabs>


Next, install the Aptos CLI.
