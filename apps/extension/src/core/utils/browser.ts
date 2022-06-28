// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

export default class Browser {
  public static storage() {
    if ((!process.env.NODE_ENV || process.env.NODE_ENV === 'development')) {
      return null;
    }
    return chrome.storage.local;
  }
}
