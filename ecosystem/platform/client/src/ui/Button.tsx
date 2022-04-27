// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import * as React from "react";

type ButtonProps = React.DetailedHTMLProps<
  React.ButtonHTMLAttributes<HTMLButtonElement>,
  HTMLButtonElement
>;

export const Button: React.FC<ButtonProps> = ({children, ...rest}) => {
  return (
    <button
      className="inline-flex justify-center py-2 px-4 border border-transparent shadow-sm text-sm font-medium rounded-md text-white bg-indigo-600 hover:bg-indigo-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-indigo-500"
      {...rest}
    >
      {children}
    </button>
  );
};
