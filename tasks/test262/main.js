// @ts-check

import { shakeSingleModule } from 'jsshaker'
import { readFileSync, writeFileSync } from 'fs'

const input = readFileSync('./input.js', 'utf8');

const start = Date.now();
const result = shakeSingleModule(input, {
  preset: 'recommended',
});
console.log('Time:', Date.now() - start + 'ms');

writeFileSync('./output.js', result.output.code);
writeFileSync('./diagnostics.txt', result.diagnostics.join('\n'));
