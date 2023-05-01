---
title: "Unity SDK"
slug: "unity-sdk"
---


# Aptos Unity SDK

The [Aptos Unity SDK](https://github.com/aptos-labs/Aptos-Unity-SDK) is a .NET implementation of the [Aptos SDK](./index.md), compatible with .NET Standard 2.0 and .NET 4.x for Unity. The goal of this SDK is to provide a set of tools for developers to build multi-platform applications (mobile, desktop, web, VR) using the Unity game engine and the Aptos blockchain infrastructure.

See the post [Aptos Labs brings Web3 to Gaming with its new SDK for Unity developers](https://medium.com/aptoslabs/aptos-labs-brings-web3-to-gaming-with-its-new-sdk-for-unity-developers-e6544bdf9ba9) and the [Technical details](https://github.com/aptos-labs/Aptos-Unity-SDK#technical-details) section of the Unity SDK README for all of the features offered to game developers by the Aptos Unity SDK.

## User flows

The Aptos Unity SDK supports these use cases:

- *Progressive onboarding flow* in which users can log into a game by email. In this flow, transactions are proxied, and Aptos uses a distributed key system. The users can then onboard to a full custodial wallet if desired.
- *In-game non-custodial wallet integration* in which game developers have the option to allow users to create full non-custodial wallets in the games.
- *Off-game non-custodial wallet integration* in which game developers may allow users to connect to a desktop wallet or a mobile wallet within the game or create burner wallets from the parent wallet seamlessly.


## Prerequisites

### Supported Unity versions
| Supported Version: | Tested |
| -- | -- |
| 2021.3.x | ✅ |
| 2022.2.x | ✅ |

| Windows | Mac  | iOS | Android | WebGL |
| -- | -- | -- | -- | -- |
| ✅ | ✅ | ✅ | ✅ | ✅ |

### Dependencies

> As of Unity 2021.x.x, Newtonsoft Json is a common dependency. Prior versions of Unity require installing Newtonsoft.

- [Chaos.NaCl.Standard](https://www.nuget.org/packages/Chaos.NaCl.Standard/)
- Microsoft.Extensions.Logging.Abstractions.1.0.0 — required by NBitcoin.7.0.22
- Newtonsoft.Json
- NBitcoin.7.0.22
- [Portable.BouncyCastle](https://www.nuget.org/packages/Portable.BouncyCastle)
- Zxing

## Install the Unity SDK

You may install the Unity SDK either through our `unitypackage` or the [Unity Package Manager](https://docs.unity3d.com/Manual/Packages.html).

### Install by `unitypackage`

1. Start Unity.
2. Download the latest `Aptos.Unity.unitypackage` file from the [Unity Asset Store](https://assetstore.unity.com/packages/decentralization/aptos-sdk-244713).
3. Click **Assets** → **Import Packages** → **Custom Package** and select the downloaded file.

### Install by Unity Package Manager

1. Open the [Unity Package Manager](https://docs.unity3d.com/Manual/upm-ui.html) window.
2. Click the add **+** button in the top status bar.
3. Select *Add package from git URL* from the dropdown menu.
4. Enter the URL *https://github.com/aptos-labs/Aptos-Unity-SDK.git* and click **Add**.