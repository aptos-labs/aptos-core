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

# Production Readiness
## Profiler
```bash
yarn profile
```

## [GENERATE_SOURCEMAP=false](https://dev.to/jburroughs/don-t-use-create-react-app-until-you-know-this-1a2d)
In the `build` step in package.json I've included this to further reduce the bundle size on production builds. It removes the sourcemaps which is helpful for 
debugging in prod. By default [CRA sets it to true](https://dev.to/jburroughs/don-t-use-create-react-app-until-you-know-this-1a2d).

To include sourcemaps, run

```bash
yarn build:dev
```