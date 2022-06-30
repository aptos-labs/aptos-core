# Aptos Wallet - Browser Extension

## Setup

**A. Extension**
1. `yarn build`
2. In Chrome, go to [chrome://extensions/](chrome://extensions/)
3. Enable developer mode
4. Hit `Load Unpacked` and point to new `build` folder in this directory

Alternatively, you can download the latest release build from our [core repo](https://github.com/aptos-labs/aptos-core/releases).

**B. Webpage**
1. `yarn start`

## Linting
```bash
# Autofix all linting issues
yarn lint --fix
```
