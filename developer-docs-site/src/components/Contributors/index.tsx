import React from "react";

import CONTRIBUTORS from "../../contributors.json";

const Contributor = ({ contributor }: Contributor) => {
  const { name, email, username } = contributor;
  const avatar = username ? `https://github.com/${username}.png` : "https://github.com/identicons/aptos.png";
  const profile = username ? `https://github.com/${username}` : null;
  return (
    <a href={profile} target="_blank">
      <img className="contributor-pic" src={avatar} />
      <span style={{ marginLeft: "0.5rem" }}>{username || name}</span>
    </a>
  );
};

const Contributors = () => {
  return (
    <div className="contributors">
      <h2 className="docusaurus-mt-lg">Contributors</h2>
      <div className="contributors-list">
        {CONTRIBUTORS.map((contributor) => {
          return (
            <div key={contributor.username} className="contributor">
              <Contributor contributor={contributor} />
            </div>
          );
        })}
      </div>
    </div>
  );
};

interface Contributor {
  username: string;
  name: string;
  email: string;
}

export default Contributors;
