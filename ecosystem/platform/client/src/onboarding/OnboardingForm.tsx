import * as React from "react";
import {Persona, SocialAccount, Identity} from "./types";
import {AptosAddressInput, Button, Checkbox} from "ui";

type Props = {
  onSubmit: (identity: Identity) => void;
};

export function OnboardingForm({onSubmit}: Props) {
  const handleSubmit = (e: React.FormEvent<HTMLFormElement>) => {
    e.preventDefault();

    const formData = new FormData(e.target as HTMLFormElement);
    const {address} = Object.fromEntries(formData);
    if (typeof address !== "string") return;

    const persona: Persona = "operator";
    const account: SocialAccount = {service: "github", username: "example"};
    const identity = {
      personas: [persona],
      socialAccounts: [account],
      tosAcceptance: {date: Date.now()},
      aptosAddress: address,
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
                href="#"
              >
                Terms of Service
              </a>
              .
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
