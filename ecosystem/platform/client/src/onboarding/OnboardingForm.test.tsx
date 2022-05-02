// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

import {fireEvent, render, screen} from "@testing-library/react";
import * as React from "react";

import {OnboardingForm} from "./OnboardingForm";

it("calls onSubmit if the form is filled out and the submit button is clicked", () => {
  const onSubmit = jest.fn();
  render(<OnboardingForm onSubmit={onSubmit} />);

  const address = screen.getByRole("textbox", {name: "Address"});
  fireEvent.change(address, {target: {value: "0x1337"}});
  const tos = screen.getByRole("checkbox", {
    name: "I accept the Terms of Service",
  });
  fireEvent.change(tos, {target: {checked: true}});
  const submit = screen.getByRole("button", {name: "Submit"});
  fireEvent.click(submit);

  expect(onSubmit).toHaveBeenCalledWith({mainnetAddress: "0x1337"});
});

// TODO: Unskip once jsdom is updated to 19.0.0 (https://github.com/jsdom/jsdom/pull/3249).
it.skip("does not call onSubmit if the form is not filled out and the submit button is clicked", () => {
  const onSubmit = jest.fn();
  render(<OnboardingForm onSubmit={onSubmit} />);

  const submit = screen.getByRole("button", {name: "Submit"});
  fireEvent.click(submit);

  expect(onSubmit).not.toHaveBeenCalled();
});
