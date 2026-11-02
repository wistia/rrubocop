Run `scripts/publish-wistia.sh` from a clean, committed checkout. It builds
macOS ARM64 and static musl Linux AMD64/ARM64 binaries using a digest-pinned
cargo-zigbuild Docker image, then uploads all three tarballs with the AWS CLI.
Docker and AWS credentials with `s3:PutObject` permission on
`s3://wistia-deploy/wistia/rruboop/*` are required.

Use `scripts/publish-wistia.sh --build-only` to skip uploads. Archives are
written to `dist/rrubocop.latest.<platform>.tar.gz`, where platform is
`darwin-arm64`, `linux-amd64`, or `linux-arm64`. Each contains `rrubocop`,
`REVISION` (the full source commit), `LICENSE`, and `NOTICE`. macOS requires
Apple Silicon and macOS 11 or newer.

`RRUBOCOP_BUILD_DIR` overrides the build/cache directory (default
`target/wistia-build`). `RRUBOCOP_BUILD_IMAGE` overrides the build image and
`RRUBOCOP_S3_PREFIX` overrides the upload destination. Docker runs with the
caller's UID and mounts the source read-only.

The Wistia launcher downloads these archives from
`https://d2iok8515ir7ba.cloudfront.net/wistia/rruboop/`. Releases set a 60-second
HTTP cache lifetime. CI refreshes on every job; developers run
`bin/install-rrubocop --force` in the Wistia repository after publishing to
refresh their local installation.
