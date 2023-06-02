import React, { useEffect, useState } from "react";
import BrowserOnly from "@docusaurus/BrowserOnly";

import ReactMarkdown from "react-markdown";
import rehypeRaw from "rehype-raw";
import remarkGfm from "remark-gfm";

import ExecutionEnvironment from "@docusaurus/ExecutionEnvironment";

const root = "https://raw.githubusercontent.com/aptos-labs/aptos-core/";
const url_root = "/reference/move";

const branches = ["mainnet", "testnet", "devnet", "main"];

const branch_titles = ["Mainnet", "Testnet", "Devnet", "Main"];

const frameworks = ["move-stdlib", "aptos-stdlib", "aptos-framework", "aptos-token", "aptos-token-objects"];
const TopNav = ({ branch }: TopNavProps) => {
  const adjustBranch = (event) => {
    const params = new URLSearchParams(window.location.search);
    params.set("branch", event.target.getAttribute("branch"));
    window.location.href = `${location.pathname}?${params.toString()}`;
  };

  return (
    <div className="move-top-bar" key="move-top-bar">
      <div branch="mainnet" className="move-top-bar-button" key="mainnet" onClick={adjustBranch}>
        Mainnet
      </div>
      <div branch="testnet" className="move-top-bar-button" key="testnet" onClick={adjustBranch}>
        Testnet
      </div>
      <div branch="devnet" className="move-top-bar-button" key="devnet" onClick={adjustBranch}>
        Devnet
      </div>
      <div branch="main" className="move-top-bar-button" key="main" onClick={adjustBranch}>
        Main
      </div>
    </div>
  );
};

interface TopNavProps {
  branch: string;
}

const SideNav = ({ branch }: SideNavProps) => {
  const [content, setContent] = useState(null);
  let isMounted = true;

  useEffect(() => {
    const fetchContent = async () => {
      if (!isMounted) {
        return;
      }

      let navbar_contents = [];

      for (const framework of frameworks) {
        const page = `${root}/${branch}/aptos-move/framework/${framework}/doc/overview.md`;
        const response = await fetch(page);
        if (response.ok) {
          const raw_content = await response.text();
          const links_regex = /\[(.+)\]\(([^ ]+?)( "(.+)")?\)/g;

          let framework_content = [];
          for (const entry of raw_content.matchAll(links_regex)) {
            const name = entry[1].replaceAll("`", "");
            const url = `${url_root}?branch=${branch}&page=${framework}/doc/${entry[2]}`;

            framework_content.push(
              <div key={name}>
                <a href={url}>{name}</a>
              </div>,
            );
          }

          navbar_contents.push(
            <div key={framework}>
              <div key={`${framework}.name`}>{framework}</div>
              <div key={`${framework}.content`}>{framework_content}</div>
            </div>,
          );
        }
      }

      const new_content = <div>{navbar_contents}</div>;
      if (isMounted) {
        setContent(new_content);
      }
    };

    fetchContent().catch((err) => console.log(`Error fetching spec: ${err}`));
    return () => {
      isMounted = false;
    };
  }, []);

  return (
    <div className="move-sidebar" key="move-sidebar">
      {content}
    </div>
  );
};

interface SideNavProps {
  branch: string;
}

const Content = ({ branch, page }: ContentProps) => {
  const [content, setContent] = useState(null);
  let isMounted = true;

  useEffect(() => {
    const fetchContent = async () => {
      if (!isMounted) {
        return;
      }
      const page_root = page.match(".*/doc")[0];

      const page_path = `${root}/${branch}/aptos-move/framework/${page}`;
      const response = await fetch(page_path);
      const raw_content = await response.text();

      const regex_major = /href="[\w\-\/\.]*(\/[\w\-]+\/doc\/)([\w\-]+.md.*)"/g;
      const regex_local = /href="([\w\-]+\.md)/g;
      const regex_minor = /page=([\w\-]+\.md)/g;
      const regex_markdown = /\(([\w\-]+\.md.*)\)/g;
      let redirected = raw_content.replaceAll(regex_major, `href="${url_root}?branch=${branch}&page=$1$2"`);
      redirected = redirected.replaceAll(regex_local, `href="/reference/move?branch=${branch}&page=$1`);
      redirected = redirected.replaceAll(regex_minor, `branch=${branch}&page=${page_root}/$1`);
      redirected = redirected.replaceAll(regex_markdown, `(/reference/move?branch=${branch}&page=${page_root}/$1)`);

      if (isMounted) {
        setContent(
          <ReactMarkdown
            children={redirected}
            rehypePlugins={[rehypeRaw]}
            remarkPlugins={[remarkGfm]}
            remarkRehypeOptions={{ allowDangerousHtml: true }}
          />,
        );
      }
    };

    fetchContent().catch((err) => console.log(`Error fetching spec: ${err}`));
    return () => {
      isMounted = false;
    };
  }, []);

  return (
    <div className="move-content" key="move-content">
      {content}
    </div>
  );
};

interface ContentProps {
  branch: string;
  page: string;
}

const Main = () => {
  let branch = "main";
  let page = null;
  if (ExecutionEnvironment.canUseViewport) {
    const params = new URLSearchParams(location.search);
    page = params.get("page") ?? "aptos-framework/doc/overview.md";
    branch = params.get("branch") ?? "mainnet";
  }

  return (
    <BrowserOnly fallback={<div>Loading...</div>}>
      {() => {
        return (
          <div className="move-reference-body" key="move-reference-body">
            <TopNav branch={branch} />
            <div className="move-reference-contents" key="move-reference-contents">
              <SideNav branch={branch} />
              <Content branch={branch} page={page} />
            </div>
          </div>
        );
      }}
    </BrowserOnly>
  );
};

export default Main;
