import * as React from "react";
import {LinkAccountsStep} from "./LinkAccountsStep";
import {TermsOfServiceStep} from "./TermsOfServiceStep";
import {AptosAddressStep} from "./AptosAddressStep";
import {SelectPersonaStep} from "./SelectPersonaStep";
import {OnboardingCompleteStep} from "./OnboardingCompleteStep";
import {Identity, isValidIdentity} from "./types";

export function OnboardingFlow() {
  const [identity, setIdentity] = React.useState<Partial<Identity>>({});

  const handleSelectPersona = (personas: Identity["personas"]) => {
    setIdentity({...identity, personas});
  };

  const handleLinkAccounts = (socialAccounts: Identity["socialAccounts"]) => {
    setIdentity({...identity, socialAccounts});
  };

  const handleTermsOfService = (tosAcceptance: Identity["tosAcceptance"]) => {
    setIdentity({...identity, tosAcceptance});
  };

  const handleAptosAddress = (aptosAddress: Identity["aptosAddress"]) => {
    setIdentity({...identity, aptosAddress});
  };

  if (isValidIdentity(identity)) {
    return <OnboardingCompleteStep identity={identity} />;
  } else {
    return (
      <>
        <SelectPersonaStep onComplete={handleSelectPersona} />
        {identity.personas != null && (
          <LinkAccountsStep onComplete={handleLinkAccounts} />
        )}
        {identity.socialAccounts != null && (
          <TermsOfServiceStep onComplete={handleTermsOfService} />
        )}
        {identity.tosAcceptance != null && (
          <AptosAddressStep onComplete={handleAptosAddress} />
        )}
      </>
    );
  }
}
