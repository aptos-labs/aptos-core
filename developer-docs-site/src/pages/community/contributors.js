import React from "react";
import Layout from "@theme/Layout";

import CONTRIBUTORS from "../../contributors.json";

const Contributor = ({ contributor }) => {
  const { username, name, email } = contributor;
  const avatar = username ? `https://github.com/${username}.png` : "https://github.com/identicons/aptos.png";
  const profile = username ? `https://github.com/${username}` : null;
  return (
    <a href={profile} target="_blank">
      <img className="contributor-pic" src={avatar} />
      <span style={{ marginLeft: "0.5rem" }}>{username || name}</span>
    </a>
  );
};

export default function Contributors() {
  return (
    <Layout title="Contributors" description="List of all Contributors">
      <div className="contributors">
        <h2 className="docusaurus-mt-lg">Contributors</h2>
        <div className="contributors-list">
          {CONTRIBUTORS.map((contributor) => {
            return (
              <div className="contributor">
                <Contributor contributor={contributor} />
              </div>
            );
          })}
        </div>
      </div>
    </Layout>
  );
}
