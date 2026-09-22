# Third-Party Notices

Animi reproduces the following text artwork directly in its asynchronous state notices. The
packages themselves are not included as application dependencies.

## catlock 0.1.8

- Artwork: `cat7` (empty state)
- Author: Radim Kolar
- Source: <https://docs.rs/crate/catlock/0.1.8>
- License: CC0-1.0 OR Unlicense

## cat-ascii-faces 2.0.0

- Artwork: `ฅ(´・ω・｀)ฅ` (stale state)
- Author: Melanie Cebula
- Source: <https://www.npmjs.com/package/cat-ascii-faces>
- Published license: BSD

## ascii-cats 1.1.1

- Artwork: `mini` (offline state) and `ready-to-attack` (error state)
- Author: William Li
- Source: <https://www.npmjs.com/package/ascii-cats>
- Published license: MIT

## Version 1.1 desktop dependencies

No runtime dependency was added for Linux or manga completion. Versions below are resolved from the checked-in lockfile; upstream licenses remain applicable to their respective components.

| Component           | Version | License           |
| ------------------- | ------- | ----------------- |
| Tauri               | 2.11.5  | MIT OR Apache-2.0 |
| Tauri build         | 2.6.3   | MIT OR Apache-2.0 |
| Tauri dialog plugin | 2.7.1   | MIT OR Apache-2.0 |
| chrono              | 0.4.45  | MIT OR Apache-2.0 |
| reqwest             | 0.12.28 | MIT OR Apache-2.0 |
| serde               | 1.0.228 | MIT OR Apache-2.0 |
| serde_json          | 1.0.150 | MIT OR Apache-2.0 |
| sqlx                | 0.8.6   | MIT OR Apache-2.0 |
| tempfile            | 3.27.0  | MIT OR Apache-2.0 |
| thiserror           | 2.0.18  | MIT OR Apache-2.0 |
| tokio               | 1.52.3  | MIT               |
| uuid                | 1.24.0  | MIT OR Apache-2.0 |

Linux uses the distribution's dynamically linked GTK 3 and WebKitGTK 4.1, rather than redistributing a browser runtime. Their license and copyright records are supplied by the installed Ubuntu packages under `/usr/share/doc`. WebKitWebDriver and fakeroot are development-only acceptance tools, downloaded/installed separately; they are not application dependencies or bundled binaries.
