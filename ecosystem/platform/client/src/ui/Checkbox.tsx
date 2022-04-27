// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import * as React from "react";

export function Checkbox({...rest}) {
  return (
    <input
      className="focus:ring-indigo-500 h-4 w-4 text-indigo-600 border-gray-300 rounded mr-2 cursor-pointer"
      type="checkbox"
      {...rest}
    />
  );
}
