# Maintaining GEditor releases

Application: https://github.com/gromilicx/Geditor  
Homebrew tap: https://github.com/gromilicx/homebrew-tap

## Build matrix

| Target | GitHub runner | Archive |
|---|---|---|
| aarch64-apple-darwin | macos-14 | macOS Apple Silicon tar.gz |
| x86_64-unknown-linux-gnu | ubuntu-22.04 | Linux x64 tar.gz |
| aarch64-unknown-linux-gnu | ubuntu-22.04-arm | Linux ARM64 tar.gz |
| x86_64-pc-windows-msvc | windows-2022 | Windows x64 ZIP |
| aarch64-pc-windows-msvc | windows-11-arm | Windows ARM64 ZIP |

Release builds use Rust 1.97.1, the committed Cargo.lock, native OS/CPU runners, and the native C toolchain. Windows releases request a static C runtime. macOS deployment target is 11.0. Linux GNU packages require glibc 2.35+ and are not Alpine/musl binaries. Intel Mac and 32-bit ARM targets are not included.

## Prepare the next version

1. Update `[package].version` in Cargo.toml and run Cargo to update Cargo.lock.
2. Run `./scripts/build.sh` and `python3 -m unittest discover -s scripts -p 'test_release.py' -v`.
3. Commit and push to main. CLI version output and About use the Cargo package version.
4. Optionally run the five-platform build without creating a release:

```sh
gh workflow run release.yml --repo gromilicx/Geditor --ref main
```

Workflow-dispatch runs create Actions artifacts only. The release script checks the executable format and CPU architecture, the version, and all archive hashes. Packaging tests use temporary synthetic file headers only for testing rejection/metadata; those fixtures are never release binaries.

## Tag and draft release

For version 0.3.0:

```sh
git tag v0.3.0
git push origin v0.3.0
```

Use a new tag matching the actual package version for future releases. Do not move an existing published release tag.

The tag workflow runs formatting, tests, Clippy, compilation, and executable `--version`/`--help` checks for every platform. After all five pass, it verifies the archives, generates `geditor.rb` and `SHA256SUMS`, and creates a draft GitHub Release. An existing published release is never overwritten by the workflow.

The release contains five archives, per-target JSON manifests, `SHA256SUMS`, and the generated Homebrew formula. Unix archives contain `geditor` and a `ge` symlink; Windows archives contain `geditor.exe` and `ge.exe`. README and license files are included.

Native CLI/unit tests are not a substitute for interactive checks of every terminal emulator. The application has additional local pseudo-terminal tests for editing, search, shell responsiveness, settings and Markdown preview; paid AI calls are not made by those tests.

## Publish and update Homebrew

Review the completed build checks and draft assets, then publish:

```sh
gh release edit v0.3.0 --repo gromilicx/Geditor --draft=false
```

Download the generated formula from that exact release:

```sh
gh release download v0.3.0 --repo gromilicx/Geditor --pattern geditor.rb --dir /path/to/release-assets
```

Copy it into the tap's `Formula/geditor.rb`, commit and push. Do not change a URL while retaining the previous archive's checksum. The formula selects the native Mac ARM64 or Linux x64/ARM64 package and creates both commands. It does not install Rust.

Validate the published tap:

```sh
brew install gromilicx/tap/geditor
brew test gromilicx/tap/geditor
ge --version
```

For upgrades:

```sh
brew update
brew upgrade geditor
```

After migrating from a manual installation, inspect `type -a ge` and `type -a geditor`; a binary in `~/.local/bin` may take precedence over Homebrew. Settings are stored outside the installation directory and should not be included in release archives.

## Signing and permissions

The GitHub CLI credential used to push workflow files needs the `workflow` scope in addition to repository access. The workflow's own token has read-only contents access except for the draft-release job, which has `contents: write`. Creating or updating a separate Homebrew repository is a maintainer operation, not an automatic workflow credential grant.

Archives are currently not Apple Developer ID/notarized or Windows code-signed releases. Adding platform signing requires the maintainer's corresponding certificates and credentials.

## References

- [Rust platform support](https://doc.rust-lang.org/rustc/platform-support.html)
- [GitHub hosted runners](https://docs.github.com/en/actions/reference/runners/github-hosted-runners)
- [GitHub Releases](https://docs.github.com/en/repositories/releasing-projects-on-github/managing-releases-in-a-repository)
- [Homebrew taps](https://docs.brew.sh/How-to-Create-and-Maintain-a-Tap)
- [Homebrew formula cookbook](https://docs.brew.sh/Formula-Cookbook)
