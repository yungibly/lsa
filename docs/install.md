# Install, build and release

## Homebrew

```sh
brew install yungibly/tap/lsa
lsa --version
lsa
```

The formula installs a prebuilt binary; Rust is not required. Releases cover Apple
Silicon and Intel macOS (Sonoma 14 or later), plus ARM64 and x86-64 Linux using
static musl binaries. The release workflow tests each package on a native GitHub
runner. Homebrew installation and the formula test run on both macOS architectures
and x86-64 Linux before the tap is updated; ARM64 Linux gets native binary/package
tests but does not yet have a Homebrew installation check.

Use `brew upgrade lsa` to update and `brew uninstall lsa` to remove it.
The binary is named `lsa`. To use it as `ls`, optionally add `alias ls=lsa` to your
own shell configuration. Building and installing never change shell aliases.

## Standalone packages

Download the matching `.tar.gz` and its `.tar.gz.sha256` from
[GitHub Releases](https://github.com/yungibly/lsa/releases). From the download
directory, verify with `shasum -a 256 -c FILE.tar.gz.sha256` on macOS or
`sha256sum -c FILE.tar.gz.sha256` on Linux. Extract the archive and copy `bin/lsa`
into a directory on PATH. No companion data files, image program, cache or service
is required. Archives include help, usage and compiler/target information.

Removing that binary uninstalls it. Explicitly configured thumbnail caches are
independent; `lsa --cache-dir=PATH --clear-cache` clears one selected cache.
Releases do not yet include developer signing or notarization.

## Local builds and packages

```sh
export CARGO_HOME="$PWD/.cargo-home"
cargo build --release --locked
./target/release/lsa --version
python3 scripts/package.py
```

Cargo declares Rust 1.88 minimum. The package script needs Python 3.12+ and writes
archives/checksums under `target/packages/`. By default it builds for `rustc`'s
host triple. `--target TRIPLE` selects another installed target, but the resulting
binary must run on the packaging machine: the script extracts it to a fresh prefix
and checks its version and complete piped mixed-directory output.

Keep Cargo storage and test artifacts inside the repository. To try a local build
as `ls`, use `alias ls="$PWD/target/release/lsa"` in the current shell.

## GitHub Actions

`CI` runs on main-branch pushes, pull requests and manual dispatch. It is also
reused by the release workflow. Four native runners run formatting, strict clippy,
Rust tests, release builds, 183 headless terminal scenarios, and extracted-package
checks. Linux uses the native musl linker. A separate job tests all Rust targets
with Rust 1.88.0 and validates the release tooling. CI archives are downloadable
from the workflow run for 14 days.

These checks verify application behavior and terminal bytes, not visible graphics.
The user still handles real Ghostty checks; see [compatibility](compatibility.md).

`Release` runs when a stable `vMAJOR.MINOR.PATCH` tag is pushed. It requires the tag
to equal Cargo.toml's version, then runs all of CI. It verifies every archive's
checksum, generates a formula from those exact files, uploads a draft GitHub
release, and publishes it once its assets are present. After the Homebrew install
tests pass, it commits `Formula/lsa.rb` to `yungibly/homebrew-tap`.

The `BREWTAP_TOKEN` Actions secret in `yungibly/lsa` supplies Contents write access
to the tap. The built-in GitHub token publishes releases. Local `.env` is ignored;
GitHub does not load it. The token must also be stored as the repository Actions
secret. Only the tap-update job receives it. Third-party actions are pinned to
verified full commit IDs; review and refresh those pins when updating Actions.

For subsequent releases:

1. Update the package version in Cargo.toml, refresh Cargo.lock with Cargo, and
   commit the changes with any release documentation.
2. Push main, then create and push a matching tag, for example:
   `git tag v0.1.1` followed by `git push origin v0.1.1`.
3. Check the Release workflow, including the final tap update. If a downstream
   install/tap job fails after publication, use GitHub's **Re-run failed jobs** so
   the published archives remain unchanged. A failed draft upload can be recovered
   by deleting the incomplete draft and rerunning the publish job.

Prerelease tags are rejected to avoid upgrading the stable Homebrew formula to an
experimental release. Release runs are serialized; existing public release assets
are never overwritten by this workflow. Publish fixes under a new version.
