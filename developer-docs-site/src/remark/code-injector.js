// const visitChildren = import("unist-util-visit-children");
const fs = require("fs");

/**
 * This allows injecting code from files so we only need to keep one copy of the code.
 * Used as such, from within any markdown file:
 * ```python
 * :!: static/examples/python/first_transaction.py section_6
 * ```
 *
 * and a `first_transaction.py` with the following contents:
 * #:!:>section_6
 * def my_meth():
 *    return "sup"
 * #<:!:section_6
 *
 * It will pull in all code between the following indicators: (it works for both # and // style comments)
 * #:!:>section_6
 * #<:!:section_6
 *
 * So, after the tree is parsed, it would be as if you had the following in your markdown:
 * ```python
 * def my_meth():
 *    return "sup"
 * ```
 */

const startTag = " ?:!:>";
const endTag = " ?<:!:";

const plugin = (options) => {
  return async (ast) => {
    const visit = await import("unist-util-visit");
    visit.visit(ast, "code", (node) => {
      if (node.value && node.value.startsWith(":!:")) {
        const parts = node.value.split(" ");
        if (parts.length !== 3) {
          throw new Error(`Correct format is ":!: file_path section_name", but got: ${node.value}`);
        }
        const [_, filepath, sectionName] = parts;
        const fileContent = readFile(filepath).toString();

        let matches;
        const re = new RegExp(startTag + sectionName + "\n?(.*?)\n?(//)?(#)?s*" + endTag + sectionName, "s");
        matches = fileContent.match(re);

        if (!matches) {
          throw new Error(`Could not find open/closing tags for section '${sectionName}' in ${filepath}`);
        }
        // Remove line breaks from start/end, but not whitespace
        let code = matches[1].replace(/^[\r\n]+|[\r\n]+$/g, "");

        // Remove leading whitespaces, but keep all rows aligned
        // 1) Split the lines, 2) Find the shortest indented code, 3) Ignore empty lines
        // 4) Strip the lines, 5) Join
        let split_lines = code.split(/\r?\n/);
        let minimum = null;
        for (let line of split_lines) {
          let whitespace = line.match(/^\s+/g);
          if (whitespace === null) {
            if (line.length === 0) {
              continue;
            }
            minimum = 0;
            break;
          } else if (whitespace[0].length < minimum || minimum === null) {
            minimum = whitespace[0].length;
          }
        }
        let stripped_lines = [];
        for (let line of split_lines) {
          stripped_lines.push(line.substr(minimum));
        }
        node.value = stripped_lines.join("\n");
      }
    });
  };
};

function readFile(filepath) {
  try {
    return fs.readFileSync(filepath, { encoding: "utf8" });
  } catch (e) {
    throw new Error(`Failed to read file: ${filepath}`);
  }
}

module.exports = plugin;
