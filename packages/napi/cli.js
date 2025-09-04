#!/usr/bin/env node

const { parseArgs } = require("node:util");
const { treeShakeEntry } = require("./index.js");
const { writeFile, mkdir } = require("node:fs/promises");
const { join, dirname } = require("node:path");

const { values, positionals } = parseArgs({
  options: {
    preset: {
      type: "string",
      short: "p",
    },
    minify: {
      type: "boolean",
      short: "m",
    },
    outdir: {
      type: "string",
      short: "o",
    },
  },
  allowPositionals: true,
  strict: false,
});

if (positionals.length !== 1) {
  throw new Error("Must provide exactly one entry js file path.");
}

const result = treeShakeEntry(
  positionals[0],
  values.preset || "recommended",
  values.minify || false
);

for (const message of result.diagnostics) {
  console.warn(message);
}

for (let [path, content] of Object.entries(result.output)) {
  if (values.outdir) {
    path = join(values.outdir, path);
  }
  const dir = dirname(path);
  mkdir(dir, { recursive: true });
  console.log(path);
  writeFile(path, content);
}
