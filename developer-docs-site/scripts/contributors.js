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
  "aching@calibra.com": "aching",
  "bo@aptoslabs.com": "areshand",
  "christian@aptoslabs.com": "geekflyer",
  "jijunleng@gmail.com": "jjleng",
  "josh.lind@hotmail.com": "joshlind",
  "kent@aptoslabs.com": "kent-white",
  "kevin@aptoslabs.com": "movekevin",
  "max@aptoslabs.com": "capcap",
  "msmouse@gmail.com": "msmouse",
  "raj@aptoslabs.com": "rajkaramchedu",
  "wgrieskamp@gmail.com": "wrwg",
});

// Fetch the token for using the GitHub GraphQL API. First try the environment (for CI)
// and if that doesn't work, try to use the GH CLI (for local use).
function getGitHubToken() {
  const { GITHUB_TOKEN } = process.env;
  if (GITHUB_TOKEN) {
    console.log("Using token from the GITHUB_TOKEN environment variable");
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
async function fetchEmailToUsername() {
  // Read contributor emails from the git log and store them in an array.
  const out = shell.exec('git log --format="%ae" | sort -u', { silent: true });
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
      query += ` query${idx}: search(query: "in:email ${email}", type: USER, first: 1) { nodes { ... on User { login email } } }`;
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
    const nodes = Object.values(responseBody.data).flatMap((value) => value.nodes);

    for (let i = 0; i < nodes.length; i++) {
      const { email, login } = nodes[i];
      if (!email) {
        continue;
      }
      emailUsernameMap.set(email.toLowerCase(), login);
    }

    console.log(`Fetched ${page + emailChunk.length} usernames out of ${emails.length} emails`);
  }

  return emailUsernameMap;
}

const GITHUB_USERS_EMAIL_REGEX = /(\d+\+)?([^@]+)@users\.noreply\.github\.com/;

const lookupEmailToUsername = (email, emailToUsername) => {
  email = email.toLowerCase();
  if (ADDITIONAL_EMAIL_TO_USERNAME[email]) {
    return ADDITIONAL_EMAIL_TO_USERNAME[email];
  } else if (emailToUsername.has(email)) {
    return emailToUsername.get(email);
  } else if (GITHUB_USERS_EMAIL_REGEX.test(email)) {
    return email.match(GITHUB_USERS_EMAIL_REGEX)[2];
  }
  return null;
};

const resolveContributors = (contributors, emailToUsername) => {
  const result = [];

  // Group by email.
  const emails = {};
  for (const contributor of contributors) {
    if (emails[contributor.email]) {
      continue;
    }
    emails[contributor.email] = true;
    result.push(contributor);
  }

  // Group by name.
  const names = {};
  for (const contributor of result.slice()) {
    if (names[contributor.name]) {
      if (names[contributor.name].username == null) {
        names[contributor.name].username = lookupEmailToUsername(contributor.email, emailToUsername);
      }
      result.splice(result.indexOf(contributor), 1);
      continue;
    }
    names[contributor.name] = contributor;
  }

  return result;
};

async function contributorsForFile(filePath, emailToUsername) {
  const shortlog = shell.exec(`git shortlog -sne -- "${path.basename(filePath)}" < /dev/tty`, {
    cwd: path.dirname(filePath),
    silent: true,
  });

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

async function urlPath(docRoot, docPath) {
  const relativePath = path.relative(docRoot, docPath);
  const urlRoot = relativePath.includes("/") ? "/" + path.dirname(relativePath) + "/" : "/";
  const docContents = await fs.readFile(docPath, "utf8");
  const slugMatch = docContents.match(/^slug:\s*["']?([^"']+)["']?$/im);
  if (slugMatch) {
    return urlRoot + slugMatch[1];
  } else {
    const filename = path.basename(relativePath);
    return urlRoot + filename.substring(0, filename.length - 3); // Remove .md suffix
  }
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
  const emailToUsername = await fetchEmailToUsername();
  const result = {};
  const docRoot = path.join(__dirname, "../docs");
  for await (const docPath of docPaths(docRoot)) {
    const url = await urlPath(docRoot, docPath);
    result[url] = await contributorsForFile(docPath, emailToUsername);
    console.log("Determining contributors for", url);
  }
  const json = JSON.stringify(result, null, 2);
  await fs.writeFile(path.join(__dirname, "../src/contributors.json"), json);
  console.log("Done!!");
})();
