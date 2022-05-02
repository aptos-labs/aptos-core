// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import {render, screen} from "@testing-library/react";
import * as React from "react";

import {AptosAddressInput} from "./AptosAddressInput";

it("accepts valid addresses", () => {
  render(
    <form>
      <AptosAddressInput value="0x1337" onChange={jest.fn()} />
    </form>,
  );
  const input = screen.getByRole("textbox");
  expect(input.matches(":valid")).toBe(true);
});

it("rejects strings without 0x", () => {
  render(
    <form>
      <AptosAddressInput value="1337" onChange={jest.fn()} />
    </form>,
  );
  const input = screen.getByRole("textbox");
  expect(input.matches(":valid")).toBe(false);
});

it("rejects non-hex strings", () => {
  render(
    <form>
      <AptosAddressInput value="0xf00bar" onChange={jest.fn()} />
    </form>,
  );
  const input = screen.getByRole("textbox");
  expect(input.matches(":valid")).toBe(false);
});

it("rejects addresses exceeding 32 bytes", () => {
  render(
    <form>
      <AptosAddressInput
        value={"0x" + new Array(34).join("1")}
        onChange={jest.fn()}
      />
    </form>,
  );
  const input = screen.getByRole("textbox");
  expect(input.matches(":valid")).toBe(false);
});
