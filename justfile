set windows-shell := ["powershell"]
set shell := ["bash", "-cu"]

_default:
  just --list -u

fix:
  cargo clippy --fix --allow-dirty --allow-staged
  cargo fix --allow-dirty --allow-staged
  cargo fmt --all

napi:
  pnpm -C ./packages/napi build
