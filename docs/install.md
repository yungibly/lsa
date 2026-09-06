# Build, package and install

lsa is a development CLI. The locally tested target is arm64 macOS with Rust 1.98.0.
Cargo declares Rust 1.88 minimum; that toolchain and Linux are still unverified.

From the repository:

```sh
export CARGO_HOME="$PWD/.cargo-home"
cargo build --release --locked
./target/release/lsa --version
python3 scripts/package.py
```

The package script needs Python 3.12+. It builds for `rustc`'s host triple using
repository-local Cargo storage/output. It writes a `.tar.gz` and `.sha256` under
`target/packages/`, containing `bin/lsa`, help, usage and toolchain identity.
It then extracts to a temporary prefix under that directory and verifies version
and complete piped mixed-directory output. Nothing is installed or published.

For the tested host, the artifact is
`target/packages/lsa-0.1.0-aarch64-apple-darwin.tar.gz`. Verify its checksum from the
archive directory with `shasum -a 256 -c FILE.tar.gz.sha256`, then extract it and
run `bin/lsa`. For personal installation, copy that binary into a directory on
PATH; no companion data files, image program, cache or service is required.
To try it as `ls`, set a shell alias to the absolute binary path. From this checkout,
`alias ls="$PWD/target/release/lsa"` affects only the current shell; put an alias in
your own shell configuration if you want it to persist. `unalias ls` restores the
normal command. No shell file is modified by building or packaging.

Removing the binary uninstalls it. Explicitly configured thumbnail caches are
independent; `lsa --cache-dir=PATH --clear-cache` clears one selected cache.

This is a host package, not cross-compilation or distribution infrastructure.
Before publishing, run the build/tests/extracted-binary checks on each advertised
OS/architecture, verify the declared Rust minimum, and record real terminal trials.
No signing, notarization, package-manager formula or public release is configured.
