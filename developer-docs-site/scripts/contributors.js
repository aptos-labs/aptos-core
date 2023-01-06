#!/usr/bin/env node

const fs = require("fs/promises");
const path = require("path");
const shell = require("shelljs");

// TODO: Generate this automatically.
const EMAIL_TO_USERNAME = Object.freeze({
  "raj@aptoslabs.com": "rajkaramchedu",
  "isaac.wolinsky@gmail.com": "davidiw",
  "sherryxiao.py@gmail.com": "sherry-x",
  "kevin@aptoslabs.com": "movekevin",
  "josh.lind@hotmail.com": "joshlind",
  "bo@aptoslabs.com": "areshand",
  "greg@gnazar.io": "gregnazario",
  "christian@aptoslabs.com": "geekflyer",
  "kent@aptoslabs.com": "kent-white",
  "danielporteous1@gmail.com": "banool",
  "jijunleng@gmail.com": "jjleng",
  "msmouse@gmail.com": "msmouse",
  "max@aptoslabs.com": "capcap",
  "rustie117@gmail.com": "rustielin",
  "z@chdenton.com": "zacharydenton",
  "jacob@blient.com": "jacobadevore",
  "davidmehi@google.com": "davidmehi",
  "wgrieskamp@gmail.com": "wrwg",
  "aching@calibra.com": "aching",
});

const GITHUB_USERS_EMAIL_REGEX = /(\d+\+)?([^@]+)@users\.noreply\.github\.com/;

const emailToUsername = (email) => {
  email = email.toLowerCase();
  if (EMAIL_TO_USERNAME[email]) {
    return EMAIL_TO_USERNAME[email];
  } else if (GITHUB_USERS_EMAIL_REGEX.test(email)) {
    return email.match(GITHUB_USERS_EMAIL_REGEX)[2];
  }
  return null;
};

const resolveContributors = (contributors) => {
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
        names[contributor.name].username = emailToUsername(contributor.email);
      }
      result.splice(result.indexOf(contributor), 1);
      continue;
    }
    names[contributor.name] = contributor;
  }

  return result;
};

async function contributorsForFile(filePath) {
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
        [name, email] = line.split(" <");
        email = email.substring(0, email.length - 1);
        const username = emailToUsername(email);
        return { name, email, username };
      }),
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
  const result = {};
  const docRoot = path.join(__dirname, "../docs");
  for await (const docPath of docPaths(docRoot)) {
    const url = await urlPath(docRoot, docPath);
    result[url] = await contributorsForFile(docPath);
  }
  const json = JSON.stringify(result, null, 2);
  await fs.writeFile(path.join(__dirname, "../src/contributors.json"), json);
})();
