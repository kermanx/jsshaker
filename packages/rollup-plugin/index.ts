import { Plugin } from "rollup";
import { Options as JsShakerOptions, shakeMultiModule } from "jsshaker";
import { createFilter, FilterPattern } from "unplugin-utils";

export interface Options {
  preset?: "safest" | "recommended" | "smallest" | "disabled";
  alwaysInlineLiteral?: boolean;
  minify?: boolean;
  showWarnings?: boolean;
  include?: FilterPattern;
  exclude?: FilterPattern;
}

export default function rollupPluginJsShaker(
  pluginOptions: Options = {},
): Plugin {
  const filter = createFilter(
    pluginOptions.include ?? /\.[mc]?js$/,
    pluginOptions.exclude,
  );

  let minify = pluginOptions.minify;
  return {
    name: "rollup-plugin-jsshaker",
    // @ts-expect-error Vite-specific hook
    apply: "build",
    // @ts-expect-error Vite-specific hook
    configResolved(config) {
      minify ??= config.build?.minify !== false;
    },
    generateBundle: {
      order: "post",
      handler(outputOptions, bundle) {
        const options: JsShakerOptions = {
          preset: pluginOptions.preset,
          alwaysInlineLiteral: pluginOptions.alwaysInlineLiteral,
          jsx: "react",
          sourceMap: !!outputOptions.sourcemap,
          minify:
            "minify" in outputOptions
              ? !!outputOptions.minify &&
                typeof outputOptions.minify === "object"
              : !!minify,
        };

        const entrySource = Object.values(bundle)
          .filter(
            (module) =>
              module.type === "chunk" &&
              module.isEntry &&
              filter(module.fileName),
          )
          .map((b) => b.fileName)
          .flatMap((name, i) => [
            `export * as e${i.toString(36)} from "./${name}";`,
            `export { default as d${i.toString(36)} } from "./${name}";`,
          ])
          .join("\n");

        const entryFileName = "___entry___";
        const sources: Record<string, string> = {
          [entryFileName]: entrySource,
        };
        for (const [fileName, module] of Object.entries(bundle)) {
          if (module.type === "chunk") {
            sources[fileName] = module.code;
          }
        }

        const startTime = Date.now();
        this.info(`Optimizing chunks...`);
        const shaken = shakeMultiModule(sources, entryFileName, options);
        this.info(`Completed in ${Date.now() - startTime} ms`);

        if (pluginOptions.showWarnings) {
          for (const diag of shaken.diagnostics) {
            this.warn(`${diag}`);
          }
        }

        delete shaken.output[entryFileName];
        const maxFileNameLength = Math.max(
          ...Object.keys(shaken.output).map((n) => n.length),
        );
        let totalOriginalSize = 0;
        let totalShakenSize = 0;
        for (const [fileName, chunk] of Object.entries(shaken.output)) {
          const module = bundle[fileName];
          if (module && module.type === "chunk") {
            const percentage = (
              (chunk.code.length / module.code.length) *
              100
            ).toFixed(2);
            this.info(
              `- ${fileName.padEnd(maxFileNameLength)}  ${percentage}% (${module.code.length} -> ${chunk.code.length} bytes)`,
            );
            totalOriginalSize += module.code.length;
            totalShakenSize += chunk.code.length;
            module.code = chunk.code;
            // if (chunk.sourceMapJson) {
            //   module.map = JSON.parse(chunk.sourceMapJson);
            // }
          } else {
            throw new Error(
              `JsShaker Vite plugin expected to find module ${fileName} in the bundle.`,
            );
          }
        }

        const totalPercentage = (
          (totalShakenSize / totalOriginalSize) *
          100
        ).toFixed(2);
        this.info(
          `${"-".repeat(maxFileNameLength - 4)} Total  ${totalPercentage}% (${totalOriginalSize} -> ${totalShakenSize} bytes)`,
        );
      },
    },
  };
}
