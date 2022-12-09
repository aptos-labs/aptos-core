import React from "react";
import Footer from "@theme-original/DocItem/Footer";
import BrowserOnly from "@docusaurus/BrowserOnly";

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
    <div class="aptos-contributors">
      <h2 class="docusaurus-mt-lg">Authors</h2>
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
  return (
    <>
      <Footer {...props} />
      <BrowserOnly>
        {() => {
          const contributorsNode = document.getElementById("aptos-doc-contributors")?.textContent;
          const contributors = contributorsNode ? JSON.parse(contributorsNode) : [];
          return contributors.length > 0 && <Contributors contributors={contributors} />;
        }}
      </BrowserOnly>
    </>
  );
}
