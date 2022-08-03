// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

export default class Browser {
  static isDev() {
    return !process.env.NODE_ENV || process.env.NODE_ENV === 'development';
  }

  public static storage() {
    if (this.isDev()) {
      return null;
    }
    return chrome.storage.local;
  }

  public static tabs() {
    if (this.isDev()) {
      return null;
    }
    return chrome.tabs;
  }
}
