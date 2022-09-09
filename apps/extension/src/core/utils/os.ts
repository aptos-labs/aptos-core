// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

/**
 * Different types of operating systems
 */
export enum OS {
  ANDROID = 'Android',
  IOS = 'iOS',
  LINUX = 'Linux',
  MAC = 'Mac OS',
  WINDOWS = 'Windows',
}

/**
 * Get the client's operating system
 * Window must be defined
 */
export function getOS(): OS | null {
  if (typeof window === 'undefined') {
    return null;
  }

  const { userAgent } = window.navigator;
  const { platform } = window.navigator;
  const macosPlatforms = ['Macintosh', 'MacIntel', 'MacPPC', 'Mac68K'];
  const windowsPlatforms = ['Win32', 'Win64', 'Windows', 'WinCE'];
  const iosPlatforms = ['iPhone', 'iPad', 'iPod'];

  if (macosPlatforms.indexOf(platform) !== -1) {
    return OS.MAC;
  } if (iosPlatforms.indexOf(platform) !== -1) {
    return OS.IOS;
  } if (windowsPlatforms.indexOf(platform) !== -1) {
    return OS.WINDOWS;
  } if (/Android/.test(userAgent)) {
    return OS.ANDROID;
  } if (/Linux/.test(platform)) {
    return OS.LINUX;
  }

  return null;
}
