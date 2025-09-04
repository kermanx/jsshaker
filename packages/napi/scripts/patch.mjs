import fs from 'fs';
import path from 'path';
import { hash } from 'ohash';
import { fileURLToPath } from 'url';

const apiUrl = "https://pkg.pr.new/"

const {
  GITHUB_REPOSITORY,
  GITHUB_RUN_ID,
  GITHUB_RUN_ATTEMPT,
  GITHUB_ACTOR_ID,
} = process.env;

const [owner, repo] = GITHUB_REPOSITORY.split("/");

const metadata = {
  owner,
  repo,
  run: Number(GITHUB_RUN_ID),
  attempt: Number(GITHUB_RUN_ATTEMPT),
  actor: Number(GITHUB_ACTOR_ID),
};

const key = hash(metadata);

const checkResponse = await fetch(new URL("/check", apiUrl), {
  method: "POST",
  body: JSON.stringify({
    owner,
    repo,
    key,
  }),
});

if (!checkResponse.ok) {
  console.error(await checkResponse.text());
  process.exit(1);
}

const { sha } = await checkResponse.json();
const tag = sha.slice(0, 7)

const jsonPath = path.join(fileURLToPath(import.meta.url), '../../package.json');
const json = JSON.parse(fs.readFileSync(jsonPath, 'utf8'));

json.optionalDependencies = {};

const targets = [
  "darwin-arm64",
  "darwin-x64",
  "linux-x64-gnu",
  "wasm32-wasi",
  "win32-x64-msvc",
]
for (const target of targets) {
  json.optionalDependencies[`@jsshaker/binding-${target}`] = `https://pkg.pr.new/kermanx/jsshaker/@jsshaker/binding-${target}@${tag}`
}

fs.writeFileSync(jsonPath, JSON.stringify(json, null, 2));
