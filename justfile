set windows-shell := ["powershell"]
set shell := ["bash", "-cu"]

_default:
  just --list -u

fix:
  cargo clippy --fix --allow-dirty --allow-staged
  cargo fix --allow-dirty --allow-staged
  cargo fmt --all
  cargo test

napi:
  pnpm -C ./packages/napi build

test262: napi
  pnpm -C ./tasks/test262 run test
