// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import * as React from "react";
import {AptosAddressInput, Button, Checkbox} from "ui";

import {Identity} from "./types";

type Props = {
  onSubmit: (identity: Identity) => void;
};

export function OnboardingForm({onSubmit}: Props) {
  const handleSubmit = (event: React.FormEvent<HTMLFormElement>) => {
    event.preventDefault();

    const formData = new FormData(event.target as HTMLFormElement);
    const {address} = Object.fromEntries(formData);
    if (typeof address !== "string") return;

    const identity = {
      mainnetAddress: address,
    };

    onSubmit(identity);
  };

  return (
    <form onSubmit={handleSubmit}>
      <div className="space-y-6">
        <div>
          <label
            htmlFor="address"
            className="block text-sm font-medium text-gray-700"
          >
            Address
          </label>
          <div className="mt-1">
            <AptosAddressInput name="address" id="address" required />
          </div>
        </div>

        <div>
          <label className="block text-sm font-medium text-gray-700">
            Terms of Service
          </label>
          <div className="mt-1 flex items-center">
            <Checkbox id="tos" name="tos" required />
            <label
              className="inline-block text-sm text-gray-500 cursor-pointer"
              htmlFor="tos"
            >
              I accept the{" "}
              <a
                className="text-indigo-500 hover:text-indigo-600 focus:outline-none rounded-md focus:underline"
                href={"#tos" /* TODO: Add real TOS link. */}
              >
                Terms of Service
              </a>
            </label>
          </div>
        </div>

        <div>
          <Button type="submit">Submit</Button>
        </div>
      </div>
    </form>
  );
}
