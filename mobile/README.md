# Aptos Wallet - Android / iOS (React Native)

## Setup React Native Enviroment
**You can follow the react native setup guide [here](https://reactnative.dev/docs/environment-setup)**

Some problems we've run into on iOS
- `watchman`: `brew install watchman` sometimes has issues. You can install with ports [here](https://ports.macports.org/port/watchman/)
- `pod install`: M1 Macs sometimes have problems with cocoapods. `sudo arch -x86_64 gem install ffi` and `arch -x86_64 pod install`

## Running the App

### Android: 
`yarn android`

### iOS
If first run or pods have been updated:
1. `cd ios`
2. `pod install`

`yarn ios`

## Linting
```bash
# Autofix all linting issues
yarn lint --fix
```
