# JsShaker

🪚 Code size optimizer for JavaScript.

https://github.com/kermanx/jsshaker

## WASM & N-API

```ts
import { shakeSingleModule, shakeMultiModule } from "jsshaker";
```

## CLI

```sh
pnpx jsshaker ./entry.js

# --preset(-p, optional): "safest" | "recommended" | "smallest" | "disabled"
# --minify(-m, optional): boolean
# --outdir(-o, optional): string
```

## Vite Plugin

```ts
import { defineConfig } from "vite";
import { shakeMultiModule } from "jsshaker";
import * as fs from "fs";

export default defineConfig({
  plugins: [
    {
      name: "jsshaker",
      enforce: "post",
      buildEnd() {
        const { output, diagnostics } = shakeMultiModule("./dist/index.js", {
          minify: true,
        })
        for (const diag of diagnostics) {
          console.error(diag);
        }
        for (const file in output) {
          fs.writeFileSync(file, output[file]);
        }
      },
    },
  ],
});
```
