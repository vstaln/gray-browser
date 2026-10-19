#!/bin/sh
here="$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)"
sys="$(uname -s)"
case "$sys" in
MINGW* | MSYS* | CYGWIN*) rel="x86_64-pc-windows-msvc/browser.exe" ;;
Darwin*)
    case "$(uname -m)" in
    arm64) rel="aarch64-apple-darwin/browser" ;;
    *) rel="x86_64-apple-darwin/browser" ;;
    esac ;;
*)
    case "$(uname -m)" in
    aarch64 | arm64) rel="aarch64-unknown-linux-musl/browser" ;;
    *) rel="x86_64-unknown-linux-musl/browser" ;;
    esac ;;
esac
if [ -x "$here/$rel" ]; then exec "$here/$rel" "$@"; fi
for cand in "$here/target/release/browser.exe" "$here/target/release/browser" "$here/target/debug/browser.exe" "$here/target/debug/browser"; do
    if [ -x "$cand" ]; then exec "$cand" "$@"; fi
done
echo "browser binary not found in $here" >&2
exit 1
