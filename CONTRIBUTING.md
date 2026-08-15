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

## Licensing & Developer Certificate of Origin (DCO)

### Why a DCO instead of copyright assignment?

This project uses the **GNU General Public License v2.0 (GPL-2.0)** to remain strictly open-source. To keep the project's provenance clean and avoid legal deadlocks down the line (e.g. an unreachable past contributor blocking a future refactor), every contribution needs a **Developer Certificate of Origin (DCO)** sign-off - the same lightweight system used by the Linux kernel, Kubernetes, and Docker.

Unlike a copyright assignment, a DCO sign-off does **not** transfer ownership of your code to the maintainer. You keep full copyright over your contribution. You're simply certifying that you wrote it (or otherwise have the right to submit it) and that you're licensing it to this project under GPL-2.0, same as everything else here.

### How to sign off

Add a `Signed-off-by` line to your commit message. The easiest way is to commit with `-s`:

```bash
git commit -s -m "Add feature X"
```

This appends a line like:

```
Signed-off-by: Your Name <your.email@example.com>
```

By adding this line, you certify the following (the standard [DCO 1.1](https://developercertificate.org/) text):

> By making a contribution to this project, I certify that:
>
> (a) The contribution was created in whole or in part by me and I have the right to submit it under the open source license indicated in the file; or
>
> (b) The contribution is based upon previous work that, to the best of my knowledge, is covered under an appropriate open source license and I have the right under that license to submit that work with modifications, whether created in whole or in part by me, under the same open source license (unless I am permitted to submit under a different license), as indicated in the file; or
>
> (c) The contribution was provided directly to me by some other person who certified (a), (b) or (c) and I have not modified it.
>
> (d) I understand and agree that this project and the contribution are public and that a record of the contribution (including all personal information I submit with it, including my sign-off) is maintained indefinitely and may be redistributed consistent with this project or the open source license(s) involved.

PRs without a `Signed-off-by` line on every commit won't be merged - if you forget, you can amend it after the fact with `git commit --amend -s` (or `git rebase --signoff <branch>` for multiple commits).

## Reporting bugs

- Check you're running the latest `mpv` and `yt-dlp` first - a surprising number of playback issues trace back to one of these being outdated.
- Include your `rs-pug` version, OS, and steps to reproduce.
- If relevant, attach a debug log: run with `--debug` to write logs to `~/.config/rs-pug/rs-pug.log`.

## Community

Questions, ideas, or just want to chat about the project? Join the [Discord](https://discord.gg/6FcBWwRQBX).