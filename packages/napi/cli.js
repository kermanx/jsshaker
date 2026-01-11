#!/usr/bin/env node

const { parseArgs } = require("node:util");
const { shakeFsModule, shakeSingleModule } = require("./index.js");
const { writeFile, mkdir, readFile } = require("node:fs/promises");
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
      single: {
        type: "boolean",
        short: "s",
      },
    },
    allowPositionals: true,
    strict: false,
  });

  if (positionals.length !== 1) {
    throw new Error("Must provide exactly one entry js file path.");
  }

  const options = {
    preset: values.preset,
    minify: values.minify,
  };

  if (!values.single) {
    const result = shakeFsModule(positionals[0], options);

    for (const message of result.diagnostics) {
      console.warn(message);
    }

    for (let [path, { code }] of Object.entries(result.output)) {
      path = join(values.outdir || "./out", path);
      const dir = dirname(path);
      await mkdir(dir, { recursive: true });
      console.log('Writing', path);
      await writeFile(path, code);
    }
  } else {
    const content = await readFile(positionals[0], "utf-8");
    const result = shakeSingleModule(content, options);

    for (const message of result.diagnostics) {
      console.warn(message);
    }

    await writeFile(values.outdir || "out.js", result.output.code);
  }
})().catch((err) => {
  console.error(err);
  process.exit(1);
});