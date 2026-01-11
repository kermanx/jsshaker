# JsShaker

🪚 Code size optimizer for JavaScript.

https://github.com/kermanx/jsshaker

## WASM & N-API

```ts
import { shakeSingleModule, shakeMultiModule, shakeFsModule } from "jsshaker";
```

## CLI

```sh
pnpx jsshaker ./entry.js

# --preset(-p, optional): "safest" | "recommended" | "smallest" | "disabled" - Preset configuration (default: "recommended")
# --minify(-m, optional): boolean - Minify output (default: false)
# --outdir(-o, optional): string  - Output directory (default: ./out)
# --single(-s, optional): boolean - Shake as a single module (default: ./out.js)
```

## Rollup/Vite/Rolldown Plugin

https://www.npmjs.com/package/rollup-plugin-jsshaker
