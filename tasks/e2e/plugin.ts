import { createLogger, Plugin } from "vite";
import pico from "picocolors";

export default function (options: {
  pre?: (code: string) => string,
  post?: (code: string) => string,
} = {}): Plugin | false {
  const logger = createLogger("info", {
    prefix: "tree-shaker"
  })

  const disabled = +(process.env.DISABLE_TREE_SHAKE ?? 0);
  const treeShake = disabled ? null : import("@kermanx/tree-shaker");

  return {
    name: "tree-shaker",
    enforce: 'post',
    apply: 'build',
    config(config) {
      return {
        build: {
          // Currently enabling Rollup treeshake because JS built-ins is not supported yet
          // rollupOptions: {
          //   treeshake: false
          // },
          outDir: './dist',
          emptyOutDir: false,
          ...config?.build,
          lib: {
            entry: './main.ts',
            formats: ['es'],
            fileName: disabled ? 'bundled' : 'shaken',
            ...config?.build?.lib,
          },
          modulePreload: {
            polyfill: false,
            ...(typeof config?.build?.modulePreload === 'object' ? config.build.modulePreload : {}),
          },
        },
        esbuild: {
          minifyIdentifiers: false,
          minifyWhitespace: false,
          minifySyntax: false,
        },
      }
    },
    renderChunk: {
      order: 'post',
      async handler(code) {
        if (disabled) {
          return code;
        }
        code = options.pre?.(code) ?? code;
        const startTime = Date.now();
        const { output, diagnostics } = (await treeShake).treeShake(code, "recommended", false);
        const duration = `${Date.now() - startTime}ms`;
        logger.info(pico.yellowBright(`\ntree-shake duration: ${duration}`));
        for (const diagnostic of diagnostics) {
          logger.error(diagnostic);
        }
        return options.post?.(output) ?? output;
      }
    }
  }
}
