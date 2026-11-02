#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/.."
upload=true
case "${1:-}" in
  --build-only) upload=false ;;
  "") ;;
  *) echo "Usage: $0 [--build-only]" >&2; exit 2 ;;
esac

: "${RRUBOCOP_BUILD_IMAGE:=ghcr.io/rust-cross/cargo-zigbuild@sha256:eb5578cfc06e070a09deb7b5852474bd128a4e561f9f123261b5ed2405e478b1}"
: "${RRUBOCOP_S3_PREFIX:=s3://wistia-deploy/wistia/rruboop}"
: "${RRUBOCOP_BUILD_DIR:=$PWD/target/wistia-build}"
revision=$(git rev-parse HEAD)
if [[ -n $(git status --porcelain) ]]; then
  echo "Commit tracked changes before publishing so archives have an identifiable revision." >&2
  exit 1
fi
mkdir -p dist "$RRUBOCOP_BUILD_DIR/cargo"
RRUBOCOP_BUILD_DIR=$(cd "$RRUBOCOP_BUILD_DIR" && pwd)
stage=$(mktemp -d)
trap 'rm -rf "$stage"' EXIT

for platform in darwin-arm64 linux-amd64 linux-arm64; do
  case "$platform" in
    darwin-arm64) target=aarch64-apple-darwin ;;
    linux-amd64) target=x86_64-unknown-linux-musl ;;
    linux-arm64) target=aarch64-unknown-linux-musl ;;
  esac
  docker run --rm --user "$(id -u):$(id -g)" \
    -e HOME=/tmp -e CARGO_HOME=/build/cargo -e CARGO_TARGET_DIR=/build/target \
    -e MACOSX_DEPLOYMENT_TARGET=11.0 \
    -e "RRUBOCOP_BUILD_SHA=$revision" \
    -v "$PWD:/workspace:ro" -v "$RRUBOCOP_BUILD_DIR:/build" \
    -w /workspace "$RRUBOCOP_BUILD_IMAGE" \
    cargo zigbuild --release --locked --bin rrubocop --target "$target"
  cp "$RRUBOCOP_BUILD_DIR/target/$target/release/rrubocop" "$stage/rrubocop"
  cp LICENSE NOTICE "$stage/"
  printf '%s\n' "$revision" > "$stage/REVISION"
  tar -czf "dist/rrubocop.latest.$platform.tar.gz" -C "$stage" rrubocop LICENSE NOTICE REVISION
done
printf '%s\n' "$revision" > dist/rrubocop.latest.REVISION

if [[ "$upload" == true ]]; then
  for platform in darwin-arm64 linux-amd64 linux-arm64; do
    archive="rrubocop.latest.$platform.tar.gz"
    aws s3 cp "dist/$archive" "${RRUBOCOP_S3_PREFIX%/}/$archive" \
      --acl public-read \
      --content-type application/gzip --cache-control 'max-age=60' \
      --metadata "revision=$revision" --only-show-errors
    aws s3 cp "dist/$archive" "${RRUBOCOP_S3_PREFIX%/}/rrubocop.$revision.$platform.tar.gz" \
      --acl public-read --content-type application/gzip --cache-control 'max-age=31536000, immutable' \
      --metadata "revision=$revision" --only-show-errors
  done
  aws s3 cp dist/rrubocop.latest.REVISION "${RRUBOCOP_S3_PREFIX%/}/rrubocop.latest.REVISION" \
    --acl public-read --content-type text/plain --cache-control 'max-age=60' --only-show-errors
fi
