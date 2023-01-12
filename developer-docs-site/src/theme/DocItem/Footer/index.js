import React from "react";
import Footer from "@theme-original/DocItem/Footer";
import { useLocation } from "@docusaurus/router";
import CONTRIBUTORS from "../../../contributors.json";

const Contributor = ({ contributor }) => {
  const { username, name, email } = contributor;
  const avatar = username ? `https://github.com/${username}.png` : "https://github.com/identicons/aptos.png";
  const profile = username ? `https://github.com/${username}` : null;
  return (
    <a href={profile} target="_blank">
      <img width="32" height="32" src={avatar} />
      <span style={{ marginLeft: "0.5rem" }}>{username || name}</span>
    </a>
  );
};

const Contributors = ({ contributors }) => {
  return (
    <div className="aptos-contributors">
      <h2 className="docusaurus-mt-lg">Authors</h2>
      <div>
        {contributors.map((contributor) => {
          return (
            <div key={contributor.email} style={{ marginBottom: "1rem" }}>
              <Contributor contributor={contributor} />
            </div>
          );
        })}
      </div>
    </div>
  );
};

export default function FooterWrapper(props) {
  const location = useLocation();
  let urlPath = location.pathname;
  if (urlPath.endsWith("/")) urlPath = urlPath.substring(0, urlPath.length - 1);
  const contributors = CONTRIBUTORS[urlPath];
  return (
    <>
      <Footer {...props} />
      {contributors?.length > 0 && <Contributors contributors={contributors} />}
    </>
  );
}
