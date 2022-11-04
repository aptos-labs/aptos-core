import React from "react";
import { API } from "@stoplight/elements";
// TODO: Look into defining source order for compiling from component earlier to prevent specificity issues
// import "@stoplight/elements/styles.min.css";

const ApiExplorer = ({ network, layout }: ApiExplorerProps) => (
  <API apiDescriptionUrl={`https://fullnode.${network}.aptoslabs.com/v1/spec.yaml`} router="hash" layout={layout} />
);

interface ApiExplorerProps {
  network: string;
  layout: "sidebar" | "stacked";
}

export default ApiExplorer;
