#!/usr/bin/env bash
# Build the macOS *portable* artifact: a tarball of the .app bundle, so it can be
# unpacked and run without the .dmg installer.
#
# Usage: ./scripts/package-portable.sh v0.5.0 "--target aarch64-apple-darwin"
set -euo pipefail

tag="${1:?usage: package-portable.sh <tag> [build args]}"
args="${2:-}"

# tauri-action passes the target through --args; the bundle lands under a
# target-specific directory when one is set.
target=""
if [[ "$args" =~ --target[[:space:]]+([^[:space:]]+) ]]; then
  target="${BASH_REMATCH[1]}"
fi

arch="$(uname -m)"
if [[ -n "$target" ]]; then
  arch="${target%%-*}"   # aarch64-apple-darwin -> aarch64
fi

bundle_dir="src-tauri/target/${target:+$target/}release/bundle/macos"
app="$(find "$bundle_dir" -maxdepth 1 -name '*.app' -print -quit 2>/dev/null || true)"

if [[ -z "$app" ]]; then
  echo "::error::no .app bundle under $bundle_dir"
  exit 1
fi

mkdir -p portable
version="${tag#v}"
out="portable/GitPulse_${version}_${arch}_portable.tar.gz"
rm -f "$out"

tar -czf "$out" -C "$(dirname "$app")" "$(basename "$app")"

echo "wrote $out"
ls -l portable
