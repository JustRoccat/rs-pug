# Contributing to rs-pug

Thanks for considering a contribution! This project is maintained in spare time, so clear, focused contributions are the easiest to review and merge.

## Getting started

1. Fork the repo and clone your fork.
2. Install dependencies: `mpv` (required), `yt-dlp` (recommended).
3. Build and run:
   ```bash
   cargo build --release
   ./target/release/rs-pug
   ```
4. Make your changes on a feature branch.
5. Open a pull request against `main`.

> [!NOTE]
> Rust can fight back sometimes - if you get stuck, open a draft PR early and ask questions there.

## What's especially welcome

- **Lua plugin PRs** - new plugins for [all-rspug](https://github.com/JustRoccat/all-rspug/), or improvements to the plugin API itself. See [`docs.md`](./docs.md) for the full API reference.
- **Themes and EQ presets** - contribute these to [all-rspug](https://github.com/JustRoccat/all-rspug/) rather than this repo.
- Bug fixes, especially around playback, IPC, and the SQLite migration path.

## Scope notes

- **AUR packaging is unmaintained.** If you'd like to take it over, please open an issue rather than a PR.
- **crates.io remains actively maintained** by the author.

## Before opening a PR

- Run `cargo fmt` and `cargo clippy` and address any warnings your change introduces.
- Test manually with both a local file and a streamed source if your change touches playback.
- If you're adding a config option, document it in `README.md` (and `docs.md` if it's plugin-facing).
- Keep PRs focused - unrelated refactors make review slower and are best split out.

## Licensing & Copyright Assignment

### Why Copyright Assignment?

This project uses the **GNU General Public License v2.0 (GPL-2.0)** to remain strictly open-source and prevent corporate exploitation. However, to ensure long-term maintainability and avoid future legal deadlocks, **all contributions require a copyright assignment**.

Here is why this is necessary:
* **Unreachable Contributors:** Contributors often change emails, delete accounts, or become inactive. Without full copyright control, a single missing response from a past contributor can permanently block code refactoring, relicensing, or project restructuring years down the line.
* **Maintainer Flexibility:** It allows the maintainer to re-use or integrate components of this codebase into other projects (under any license) without needing to trace and contact every historical author.
* **Originality Assurance:** It certifies that submitted code is your original work and does not violate third-party intellectual property.

*You still keep full rights to use, modify, and distribute your original code outside of this repository.*

### Terms of Agreement

By opening a Pull Request against this repository, you explicitly agree to the following terms:

1. **Copyright Assignment:** You transfer full copyright ownership of all your submitted code and contributions in the Pull Request to the project maintainer (`JustRoccat`).
2. **Rights Retained:** You retain a perpetual, royalty-free, non-exclusive license to use, modify, and distribute your original contribution as you see fit.
3. **Relicensing Rights:** You grant the project maintainer the unrestricted right to re-license, adapt, or use your contributed code in other projects (under any license, including proprietary or permissive licenses) without requiring additional consent.
4. **Originality:** You represent that your contribution is your original work, or that you have the full right to submit it under these terms.

## Reporting bugs

- Check you're running the latest `mpv` and `yt-dlp` first - a surprising number of playback issues trace back to one of these being outdated.
- Include your `rs-pug` version, OS, and steps to reproduce.
- If relevant, attach a debug log: run with `--debug` to write logs to `~/.config/rs-pug/rs-pug.log`.

## Community

Questions, ideas, or just want to chat about the project? Join the [Discord](https://discord.gg/6FcBWwRQBX).
