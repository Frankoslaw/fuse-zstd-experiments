# fuse-zstd-experiments

Experiments with Meta's [seekable zstd](https://github.com/facebook/zstd/tree/dev/contrib/seekable_format) format: re-encode archives for random access, then mount them via FUSE without fully decompressing to disk.

## Description of the tools

zstd-seekify
---
Re-encode a plain file or a standard `.zst` archive into Meta's seekable zstd variant. The output is written next to the input as `<name>.seek.zst`. Already-seekable inputs are rejected.

**Usage**

```sh
# compress a raw disk image (writes disk.seek.zst)
cargo run --release -p zstd-seekify -- ~/backups/dell_latitude_backup.img

# re-encode an existing .zst archive at level 5 with 8 MiB frames
cargo run --release -p zstd-seekify -- ~/backups/dell_latitude_backup.zst -l 5 -f 8388608

# quiet mode (no log output, no progress bar)
cargo run --release -p zstd-seekify -- ~/backups/dell_latitude_backup.img -q
```

| Flag | Default | Description |
|------|---------|-------------|
| `-l`, `--level` | `3` | zstd compression level |
| `-f`, `--frame-size` | `4194304` (4 MiB) | seekable frame size in bytes |
| `-i`, `--input-buffer` | `16777216` (16 MiB) | read/compress buffer size |
| `-q`, `--quiet` | off | disable logging and progress bar |

archive-mount
---
Mount a seekable `.zst` file as a FUSE filesystem. The decompressed payload is exposed as a single read-only regular file, so large images (e.g. disk backups) can be accessed without storing the full uncompressed size on disk.

**Usage**

```sh
# mount with defaults (mount point: ./.archive-mount)
cargo run --release -p archive-mount -- ~/Documents/dell_latitude_backup/dell_latitude_backup.seek.zst

# custom mount point
cargo run --release -p archive-mount -- ~/backups/dell_latitude_backup.seek.zst -m /tmp/dell-mount

# verbose logging via RUST_LOG (defaults to info when unset)
RUST_LOG=debug cargo run --release -p archive-mount -- ~/backups/dell_latitude_backup.seek.zst
```

| Flag | Default | Description |
|------|---------|-------------|
| `-m`, `--mount-point` | `./.archive-mount` | directory to mount the FUSE filesystem |
| `--auto-create-mount` | `true` | create the mount point if it does not exist |
| `--auto-unmount` | `true` | unmount when the process exits |

Set `RUST_LOG` (e.g. `debug`, `trace`) to control log verbosity; `pretty_env_logger` is used for output.

## LICENSE

Copyright (c) 2026 Franciszek Łopuszański

Licensed under GNU Affero General Public License as stated in the [LICENSE](LICENSE):

```text
Copyright (c) 2026 Franciszek Łopuszański

This program is free software: you can redistribute it and/or modify it under
the terms of the GNU Affero General Public License as published by the Free
Software Foundation, either version 3 of the License, or (at your option) any
later version.

This program is distributed in the hope that it will be useful, but WITHOUT
ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS
FOR A PARTICULAR PURPOSE. See the GNU Affero General Public License for more
details.

You should have received a copy of the GNU Affero General Public License along
with this program. If not, see https://www.gnu.org/licenses/
```