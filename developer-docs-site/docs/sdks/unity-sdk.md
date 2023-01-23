---
title: "Unity SDK"
slug: "unity-sdk"
---


# Aptos Unity SDK

The Aptos Unity SDK is a .NET implementation of the [Aptos SDK](./index.md), compatible with .NET Standard 2.0 and .NET 4.x for Unity. The goal of this SDK is to provide a set of tools for developers to build multi-platform applications (mobile, desktop, web, VR) using the Unity game engine and the Aptos blockchain infrastructure.

## Core features
- HD Wallet Creation & Recovery
- Account Management
    - Account Recovery
    - Message Signing
    - Message Verification
    - Transaction Management
    - Single / Multi-signer Authentication
    - Authentication Key Rotation
- Native BCS Support
- Faucet Client for Devnet

## Prerequisites

### Unity versions
| Supported Version: | Tested |
| -- | -- |
| 2021.3.x | ✅ |
| 2022.2.x | ✅ |

| Windows | Mac  | iOS | Android | WebGL |
| -- | -- | -- | -- | -- |
| ✅ | ✅ | ✅ | ✅ | ✅ |

### Dependencies

> As of Unity 2021.x.x, Newtonsoft Json is common dependency. Prior versions of Unity require intalling Newtonsoft.

- [Chaos.NaCl.Standard](https://www.nuget.org/packages/Chaos.NaCl.Standard/)
- Microsoft.Extensions.Logging.Abstractions.1.0.0 — required by NBitcoin.7.0.22
- Newtonsoft.Json
- NBitcoin.7.0.22
- [Portable.BouncyCastle](https://www.nuget.org/packages/Portable.BouncyCastle)
- Zxing

TODO: Do we ned to guide them on finding these packages or specify verions supported?

## Install the Unity SDK

You may install the Unity SDK either through our `unitypackage` or the [Unity Package Manager](https://docs.unity3d.com/Manual/Packages.html).

### Install by `unitypackage`

1. Start Unity.
2. Download the latest `Aptos.Unity.unitypackage` file from the [Unity Asset Store](https://assetstore.unity.com/).
3. Click **Assets** → **Import Packages** → **Custom Package** and select the downloaded file.

### Install by Unity Package Manager

1. Open [Unity Package Manager](https://docs.unity3d.com/Manual/upm-ui.html) window.
2. Click the add **+** button in the top status bar.
3. Select *Add package from git URL* from the dropdown menu.
4. Enter the `https://github.com/xxxxxxxxxx.git` and click **Add**.

TODO: Explain what the GitHub URL should be above or include it directly.