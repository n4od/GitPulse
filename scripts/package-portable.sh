#!/usr/bin/env bash
# Build the macOS *portable* artifact: a tarball of the .app bundle, so it can be
# unpacked and run without the .dmg installer.
#
# The file name carries no version. The release publishes it as
# /releases/latest/download/<name>, which only resolves while the name stays the
# same from release to release; the version is inside the bundle, where Finder
# shows it.
#
# Usage: ./scripts/package-portable.sh "--target aarch64-apple-darwin"
set -euo pipefail

args="${1:-}"

# tauri-action passes the target through --args; the bundle lands under a
# target-specific directory when one is set.
target=""
if [[ "$args" =~ --target[[:space:]]+([^[:space:]]+) ]]; then
  target="${BASH_REMATCH[1]}"
fi

# One arch vocabulary across every asset name: `arm64` / `x64`. A Rust target
# triple says `aarch64` / `x86_64`, so the mapping happens here rather than
# leaking a second spelling into the release.
triple_arch="$(uname -m)"
if [[ -n "$target" ]]; then
  triple_arch="${target%%-*}"   # aarch64-apple-darwin -> aarch64
fi

case "$triple_arch" in
  aarch64) arch="arm64" ;;
  x86_64|amd64) arch="x64" ;;
  *) arch="$triple_arch" ;;
esac

bundle_dir="src-tauri/target/${target:+$target/}release/bundle/macos"
app="$(find "$bundle_dir" -maxdepth 1 -name '*.app' -print -quit 2>/dev/null || true)"

if [[ -z "$app" ]]; then
  echo "::error::no .app bundle under $bundle_dir"
  exit 1
fi

mkdir -p portable
out="portable/GitPulse-macos-${arch}-portable.tar.gz"
rm -f "$out"

tar -czf "$out" -C "$(dirname "$app")" "$(basename "$app")"

echo "wrote $out"
ls -l portable
