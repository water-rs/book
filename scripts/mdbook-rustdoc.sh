#!/bin/bash
set -euo pipefail
echo "[mdbook-rustdoc] invoked" >&2
repo_dir=$(cd "$(dirname "$0")/.." && pwd)
cd "$repo_dir"
if [ ! -d target/debug/deps ]; then
  cargo build >/dev/null
fi
crates=(waterui waterui-core waterui-navigation waterui-form waterui-media waterui-text waterui-layout)
extra_flags=()
for crate in "${crates[@]}"; do
  pattern="target/debug/deps/lib${crate//-/_}-"*.rlib
  file=$(ls $pattern 2>/dev/null | head -n1 || true)
  if [ -z "$file" ]; then
    cargo build >/dev/null
    file=$(ls $pattern 2>/dev/null | head -n1 || true)
  fi
  if [ -z "$file" ]; then
    echo "missing artifact for $crate" >&2
    exit 1
  fi
  extra_flags+=("--extern" "${crate//-/_}=$file")
done
exec rustdoc "$@" "${extra_flags[@]}"
