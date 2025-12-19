#!/usr/bin/env node

const { parseArgs } = require("node:util");
const { shakeFsModule } = require("./index.js");
const { writeFile, mkdir } = require("node:fs/promises");
const { join, dirname } = require("node:path");

(async () => {
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

  const result = shakeFsModule(
    positionals[0],
    {
      preset: values.preset,
      minify: values.minify,
    }
  );

  for (const message of result.diagnostics) {
    console.warn(message);
  }

  for (let [path, { code }] of Object.entries(result.output)) {
    if (values.outdir) {
      path = join(values.outdir, path);
    }
    const dir = dirname(path);
    await mkdir(dir, { recursive: true });
    console.log('Writing', path);
    await writeFile(path, code);
  }
})().catch((err) => {
  console.error(err);
  process.exit(1);
});