#!/usr/bin/env bash
# Build latest.json for the in-app updater from a signed release build.
#
# Run after `npm run tauri build` (with TAURI_SIGNING_PRIVATE_KEY_PATH set so
# the .sig exists), then upload latest.json AND Lineage.app.tar.gz to the
# GitHub release. The app polls:
#   https://github.com/Jam-Sw/lineage/releases/latest/download/latest.json
set -euo pipefail

REPO="Jam-Sw/lineage"
BUNDLE_DIR="src-tauri/target/release/bundle/macos"
ARCHIVE="$BUNDLE_DIR/Lineage.app.tar.gz"
SIG="$ARCHIVE.sig"

VERSION=$(node -p "require('./package.json').version")

if [[ ! -f "$ARCHIVE" || ! -f "$SIG" ]]; then
  echo "error: $ARCHIVE or its .sig is missing." >&2
  echo "Build with updater artifacts first:" >&2
  echo "  TAURI_SIGNING_PRIVATE_KEY_PATH=~/.tauri/lineage.key npm run tauri build" >&2
  exit 1
fi

SIGNATURE=$(cat "$SIG") \
VERSION="$VERSION" \
URL="https://github.com/$REPO/releases/download/v$VERSION/Lineage.app.tar.gz" \
node -e '
const { VERSION, SIGNATURE, URL } = process.env;
const manifest = {
  version: VERSION,
  pub_date: new Date().toISOString(),
  platforms: {
    "darwin-aarch64": { signature: SIGNATURE, url: URL },
  },
};
require("fs").writeFileSync("latest.json", JSON.stringify(manifest, null, 2) + "\n");
'

echo "wrote latest.json for v$VERSION"
echo "next: gh release create v$VERSION <dmg> <app.zip> \"$ARCHIVE\" latest.json ..."
