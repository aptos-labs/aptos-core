# Wallet Monorepo

This monorepo uses turborepo. We use [Yarn](https://classic.yarnpkg.com/lang/en/) as a package manager. It includes the following packages/apps:


### Getting started

1. Clone the repo
2. run `yarn install` from the root directory
3. `yarn start` or `yarn dev` in the sub-directory of your choice


### Apps and Packages

#### Apps
- `dapp-example`: a Dapp example that interacts with our wallet
- `extension`: Our wallet browser extension
- `mobile`: Our mobile react-native wallet
- [coming soon] `website`: Our wallet website

#### Packages
- `ui`: a stub React component library shared by `extension`, `website`, and `mobile` applications
- `eslint-config`: `eslint` configurations
- `tsconfig`: `tsconfig.json`s used throughout the monorepo
- [coming soon] `utils`: shared logic for CRUD operations with accounts, transactions, and more

##### FYI
1. It's important that all packages that create react components (ie. UI) should have React in `devDependencies` or `peerDependencies`, otherwise the apps that install that package will have two conflicting copies of React. 

### Build

To build all apps and packages, run the following command:

```
cd <ROOT_DIR>
yarn build
```

### Lint
To lint all apps and packages, run the following command:
```
cd <ROOT_DIR>
yarn lint
```

### Develop

To develop all apps and packages, run the following command:

```
cd <ROOT_DIR>
yarn dev
```

### Remote Caching

Turborepo can use a technique known as [Remote Caching](https://turborepo.org/docs/core-concepts/remote-caching) to share cache artifacts across machines, enabling you to share build caches with your team and CI/CD pipelines.

By default, Turborepo will cache locally. To enable Remote Caching you will need an account with Vercel. If you don't have an account you can [create one](https://vercel.com/signup), then enter the following commands:

```
cd my-turborepo
npx turbo login
```

This will authenticate the Turborepo CLI with your [Vercel account](https://vercel.com/docs/concepts/personal-accounts/overview).

Next, you can link your Turborepo to your Remote Cache by running the following command from the root of your turborepo:

```
npx turbo link
```