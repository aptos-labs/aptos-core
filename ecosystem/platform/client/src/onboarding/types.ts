export type Persona = "operator";

export type AptosAddress = string;

export type TosAcceptance = {
  date: number;
};

export type SocialAccount = {
  service: "github" | "discord";
  username: string;
};

export type Identity = {
  personas: Persona[];
  socialAccounts: SocialAccount[];
  tosAcceptance: TosAcceptance;
  aptosAddress: AptosAddress;
};

export function isValidIdentity(
  identity: Partial<Identity>,
): identity is Identity {
  return (
    identity.personas != null &&
    identity.personas.length > 0 &&
    identity.socialAccounts != null &&
    identity.socialAccounts.length > 0 &&
    identity.tosAcceptance?.date != null &&
    identity.aptosAddress != null
  );
}
