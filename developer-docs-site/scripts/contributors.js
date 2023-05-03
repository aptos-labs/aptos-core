#!/usr/bin/env node

const commandExists = require("command-exists");
const fs = require("fs/promises");
const path = require("path");
const shell = require("shelljs");
const fetch = require("node-fetch");

const PER_PAGE = 100;

// This map contains contributor contributors that are not attributed to a GitHub
// account. If you run `pnpm contributors` and notice that the username is `null`,
// you might need to go find the contributor based on their email and ask what their
// GitHub username is and add it to this map.
const ADDITIONAL_EMAIL_TO_USERNAME = Object.freeze({
  "109111707+zihan-aptos@users.noreply.github.com": "0xZihan",
  "128556004+jin-aptos@users.noreply.github.com": "0xjinn",
  "alex@alexs-macbook-pro.local": "markuze",
  "raj@aptoslabs.com": "rajkaramchedu",
  "siddharthjain@siddharths-mbp.lan": "MartianSiddharth",
});

// Fetch the token for using the GitHub GraphQL API. First try the environment (for CI)
// and if that doesn't work, try to use the GH CLI (for local use).
function getGitHubToken() {
  const { GITHUB_TOKEN } = process.env;
  if (GITHUB_TOKEN) {
    return GITHUB_TOKEN;
  }
  // If no token was provided via the environment, try to use the GH CLI.
  if (!commandExists.sync("gh")) {
    throw new Error(
      "The GITHUB_TOKEN environment variable is not set and the gh CLI is not installed, please read the README for instructions on how to fix this",
    );
  }
  // Confirm that the GH auth token used for the CLI has the necessary scopes.
  const status = shell.exec("gh auth status", { silent: true }).stderr;
  if (status.includes("not logged in")) {
    throw new Error("The GH CLI is not logged in, please run `gh auth login`");
  }
  if (!status.includes("read:user") || !status.includes("user:email")) {
    // Initiate the flow to add the necessary scopes.
    console.log("The GH CLI auth token does not have the necessary scopes. Refreshing token...");
    shell.exec("gh auth refresh --scopes read:user,user:email --hostname github.com");
  }
  const ghToken = shell.exec("gh auth token", { silent: true }).stdout.trim();
  return ghToken;
}

// Fetch the emails of all the contributors and look up their usernames. Note that it
// is not possible to look up some usernames based on the contributor email, for example
// because the contributor email no longer (or never did) map to a GitHub account, or
// because the email they use for the GitHub contributions does not match the email on
// their GitHub account. For those cases, we have a special additional mapping above.
//
// See here for where this code originally came from:
// https://stackoverflow.com/questions/75868720/how-to-lookup-github-username-for-many-users-by-email
async function fetchEmailToUsername(docRoot) {
  // Read contributor emails from the git log and store them in an array.
  const out = shell.exec(`git log --format="%aE" -- ${docRoot} | sort -u`, { silent: true });
  const emailsUnfiltered = out.stdout.split("\n").filter(Boolean);

  // Filter out emails ending with @users.noreply.github.com since the first part of
  // that email is the username.
  const emails = emailsUnfiltered.filter((email) => !email.endsWith("@users.noreply.github.com"));

  // To use the GraphQL endpoint we need to provide an auth token.
  const githubToken = getGitHubToken();

  let emailUsernameMap = new Map();

  // Break up the emails in page chunks since fetching them all at once causese
  // the query to fail.
  for (let page = 0; page < emails.length; page += PER_PAGE) {
    const emailChunk = emails.slice(page, page + PER_PAGE);

    // Build the GraphQL query string with one search query per email address in this
    // chunk. See https://docs.github.com/en/graphql/reference/queries
    let query = "query {";
    for (const [idx, email] of emailChunk.entries()) {
      query += ` query${idx}: search(query: "in:email ${email}", type: USER, first: 1) { nodes { ... on User { login } } }`;
    }
    query += " }";

    const fetchOptions = {
      method: "POST",
      headers: {
        Authorization: `token ${githubToken}`,
        "Content-Type": "application/json",
      },
      body: JSON.stringify({ query }),
    };

    const response = await fetch("https://api.github.com/graphql", fetchOptions);
    const responseBody = await response.json();

    // Parse the JSON response and append to the email => username map.
    for (const [idx, [_, value]] of Object.entries(Object.entries(responseBody.data))) {
      const email = emailChunk.at(idx);
      let login = Array.prototype.at(value.nodes, 0);
      if (!login) {
        login = await fetchEmailToUsernameViaCommit(email, docRoot);
      }
      if (login) {
        emailUsernameMap.set(email.toLowerCase(), login);
      }
    }

    console.log(`Fetched ${page + emailChunk.length} usernames out of ${emails.length} emails`);
  }

  return emailUsernameMap;
}

async function fetchEmailToUsernameViaCommit(email, docRoot) {
  const commit = shell
    .exec(`git log --author="${email}" --format="%H" --max-count=1 -- ${docRoot}`, { silent: true })
    .trim();
  if (!commit) {
    return null;
  }
  const githubToken = getGitHubToken();

  // Build the GraphQL query string with one search query per email address in this
  // chunk. See https://docs.github.com/en/graphql/reference/queries
  let query = `
    query {
      repository(owner: "aptos-labs", name: "aptos-core") {
        object(oid: "${commit}") {
          ... on Commit {
            author {
              user {
                login
              }
            }
          }
        }
      }
    }
  `;
  const fetchOptions = {
    method: "POST",
    headers: {
      Authorization: `token ${githubToken}`,
      "Content-Type": "application/json",
    },
    body: JSON.stringify({ query }),
  };

  const response = await fetch("https://api.github.com/graphql", fetchOptions);
  const responseBody = await response.json();

  return responseBody?.data?.repository?.object?.author?.user?.login;
}

const GITHUB_USERS_EMAIL_REGEX = /(\d+\+)?([^@]+)@users\.noreply\.github\.com/;

function lookupEmailToUsername(email, emailToUsername) {
  email = email.toLowerCase();
  if (ADDITIONAL_EMAIL_TO_USERNAME[email]) {
    return ADDITIONAL_EMAIL_TO_USERNAME[email];
  } else if (emailToUsername.has(email)) {
    return emailToUsername.get(email);
  } else if (GITHUB_USERS_EMAIL_REGEX.test(email)) {
    return email.match(GITHUB_USERS_EMAIL_REGEX)[2];
  }
  return null;
}

function resolveContributors(contributors, emailToUsername) {
  for (let contributor of contributors) {
    if (!contributor.username) {
      contributor.username = lookupEmailToUsername(contributor.email, emailToUsername);
    }
  }
  contributors.sort(compare_contributors);
  return contributors;
}

function compare_contributors(left, right) {
  if (left.username < right.username) {
    return -1;
  } else if (right.username < left.username) {
    return 1;
  }
  return 0;
}

function contributorsForPath(filePath, emailToUsername) {
  const shortlog = shell.exec(`git log --format="%aN <%aE>" -- "${filePath}" | sort -u`, { silent: true });
  if (shortlog.code !== 0) {
    return;
  }

  return resolveContributors(
    shortlog.stdout
      .trim()
      .split("\n")
      .map((line) => {
        line = line.trim();
        line = line.substring(line.indexOf("\t") + 1);
        let [name, email] = line.split(" <");
        email = email.substring(0, email.length - 1);
        const username = lookupEmailToUsername(email, emailToUsername);
        return { name, email, username };
      }),
    emailToUsername,
  );
}

async function* docPaths(rootDir) {
  const files = await fs.readdir(rootDir, { withFileTypes: true });

  for (const file of files) {
    const filePath = path.join(rootDir, file.name);
    if (file.isDirectory()) {
      yield* docPaths(filePath);
    } else if (file.name.endsWith(".md")) {
      yield filePath;
    }
  }
}

(async function () {
  const docRoot = path.join(__dirname, "../docs");
  const emailToUsername = await fetchEmailToUsername(docRoot);
  const result = await contributorsForPath(docRoot, emailToUsername);
  const json = JSON.stringify(result, null, 2);
  await fs.writeFile(path.join(__dirname, "../src/contributors.json"), json);
  console.log("Done!!");
})();
