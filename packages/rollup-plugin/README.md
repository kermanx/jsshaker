# roll-plugin-jsshaker

The Rollup/Rolldown/Vite plugin for [JsShaker](https://github.com/kermanx/jsshaker), an experimental code size optimizer for JavaScript based on [the Oxc parser](https://oxc.rs).

## Usage

```bash
npm install -D rollup-plugin-jsshaker
```

```js
import JsShaker from "rollup-plugin-jsshaker";

export default {
  // ...
  plugins: [JsShaker()],
};
```
