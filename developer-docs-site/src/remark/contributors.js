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

module.exports = function plugin() {
  return async (root, vfile) => {
    const shell = (await import("shelljs")).default;
    const path = await import("path");
    const shortlog = shell.exec(`git shortlog -sne -- "${path.basename(vfile.path)}" < /dev/tty`, {
      cwd: path.dirname(vfile.path),
      silent: true,
    });
    if (shortlog.code !== 0) {
      return;
    }
    const contributors = resolveContributors(
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
    root.children.push({
      type: "jsx",
      value: "<template id='aptos-doc-contributors'>{`" + JSON.stringify(contributors) + "`}</template>",
    });
  };
};
