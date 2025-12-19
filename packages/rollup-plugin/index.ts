import { Plugin } from "vite";
import { Options as JsShakerOptions, shakeMultiModule } from "jsshaker";

export interface Options {
  preset?: "safest" | "recommended" | "smallest" | "disabled";
  alwaysInlineLiteral?: boolean;
}

export default function vitePluginJsShaker(
  pluginOptions: Options = {},
): Plugin {
  return {
    name: "vite-plugin-jsshaker",
    enforce: "post",
    apply: "build",
    // config() {
    //   return {
    //     build: {
    //       rolldownOptions: {
    //         output: {
    //           advancedChunks: {
    //             groups: [
    //               {
    //                 name: "@@react@@",
    //                 test: /node_modules[\\/]react[\\/]cjs[\\/]react\./,
    //                 priority: 2,
    //               },
    //               {
    //                 name: "@@react/jsx-runtime@@",
    //                 test: /node_modules[\\/]react[\\/]cjs[\\/]react-jsx-runtime\./,
    //                 priority: 1,
    //               },
    //               {
    //                 name: "@@react-dom@@",
    //                 test: /node_modules[\\/]react-dom/,
    //               },
    //               {
    //                 name: "@@classnames@@",
    //                 test: /node_modules[\\/]classnames/,
    //               },
    //             ],
    //           },
    //         },
    //       },
    //     },
    //   };
    // },
    generateBundle: {
      order: "post",
      handler(outputOptions, bundle) {
        const options: JsShakerOptions = {
          preset: pluginOptions.preset,
          alwaysInlineLiteral: pluginOptions.alwaysInlineLiteral,
          jsx: "react",
          sourceMap: !!outputOptions.sourcemap,
          minify:
            outputOptions.minify && typeof outputOptions.minify === "object",
        };

        const entrySource = Object.values(bundle)
          .filter((module) => module.type === "chunk" && module.isEntry)
          .map((b) => b.fileName)
          .map((name) => {
            return `export * from "./${name}";\nexport { default } from "./${name}";`;
          })
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
        this.info(`[jsshaker] Optimizing chunks...`);
        const shaken = shakeMultiModule(sources, entryFileName, options);
        this.info(`[jsshaker] Completed in ${Date.now() - startTime} ms`);
        for (const diag of shaken.diagnostics) {
          this.warn(`[jsshaker] ${diag}`);
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
              `[jsshaker] ${fileName.padEnd(maxFileNameLength)} ${percentage}% (${module.code.length} -> ${chunk.code.length} bytes)`,
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
          `[jsshaker] Total: ${totalPercentage}% (${totalOriginalSize} -> ${totalShakenSize} bytes)`,
        );
      },
    },
  };
}
