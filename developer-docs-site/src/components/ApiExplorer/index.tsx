import React from "react";
import { API } from "@stoplight/elements";
import BrowserOnly from "@docusaurus/BrowserOnly";
// TODO: Look into defining source order for compiling from component earlier to prevent specificity issues
// import "@stoplight/elements/styles.min.css";

const ApiExplorer = ({ network, layout }: ApiExplorerProps) => (
  // BrowserOnly is important here because of details re SSR:
  // https://docusaurus.io/docs/advanced/ssg#browseronly
  <BrowserOnly fallback={<div>Loading...</div>}>
    {() => {
      return (
        <API
          apiDescriptionUrl={`https://raw.githubusercontent.com/aptos-labs/aptos-core/${network}/api/doc/spec.yaml`}
          router="hash"
          layout={layout}
        />
      );
    }}
  </BrowserOnly>
);

interface ApiExplorerProps {
  network: string;
  layout: "sidebar" | "stacked";
}

export default ApiExplorer;
