Run `scripts/publish-wistia.sh` from a clean, committed checkout. It builds
macOS ARM64 and static musl Linux AMD64/ARM64 binaries using a digest-pinned
cargo-zigbuild Docker image, then uploads all three tarballs with the AWS CLI.
Use `aws-vault exec wistia -- scripts/publish-wistia.sh` for Wistia credentials.
Uploads use the `public-read` ACL.
Docker and AWS credentials with `s3:PutObject` permission on
`s3://wistia-deploy/wistia/rruboop/*` are required.

Use `scripts/publish-wistia.sh --build-only` to skip uploads. Archives are
written to `dist/rrubocop.latest.<platform>.tar.gz`, where platform is
`darwin-arm64`, `linux-amd64`, or `linux-arm64`. Each contains `rrubocop`,
`REVISION` (the full source commit), `LICENSE`, and `NOTICE`. macOS requires
Apple Silicon and macOS 11 or newer.
The full source SHA is also embedded in the binary's `--version` output through
the `RRUBOCOP_BUILD_SHA` build environment variable (unmarked builds show `unknown`).

`RRUBOCOP_BUILD_DIR` overrides the build/cache directory (default
`target/wistia-build`). `RRUBOCOP_BUILD_IMAGE` overrides the build image and
`RRUBOCOP_S3_PREFIX` overrides the upload destination. Docker runs with the
caller's UID and mounts the source read-only.

The Wistia launcher downloads these archives from
`https://d2iok8515ir7ba.cloudfront.net/wistia/rruboop/`. Releases set a 60-second
HTTP cache lifetime. Each release also uploads immutable
`rrubocop.<sha>.<platform>.tar.gz` archives. Only after every archive is uploaded
does the script publish `rrubocop.latest.REVISION`, a small text file containing
the source SHA. Wistia's `./run` and CI check that marker and download an archive
only when the installed SHA differs. `bin/install-rrubocop --check-update`
checks manually; `--force` reinstalls even the current release.
