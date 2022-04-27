// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import * as React from "react";

export function AptosAddressInput({...rest}) {
  return (
    <input
      type="text"
      pattern="0x[0-9a-f]{1,32}"
      placeholder="0x1"
      className="mt-1 focus:ring-indigo-500 focus:border-indigo-500 block w-72 shadow-sm sm:text-sm border-gray-300 rounded-md font-mono"
      {...rest}
    />
  );
}
