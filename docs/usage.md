# Listings, thumbnails and limits

[Back to the README](../README.md) · [Installation](install.md) · [Compatibility](compatibility.md)

## Listings

- Terminals default to compact details: permissions, readable size, owner and
  local modification time, or a grid where images dominate. Explicit `-l`
  includes link count and group, extended-metadata indicators, and directory
  allocation totals. Directories show `-` for size; devices show major/minor
  numbers. `--header`, `--fields=LIST`, `--12-hour` and `--bytes` adjust details.
- `-C`/`-x` give columns filled down/across, including in pipes; `-1` gives
  lines, `-m` comma-separated names. The last format option wins. Use
  `-w N` or `--width=N` to choose width (1–65,535; pipes default to 80).
- `-a` includes hidden entries and `.` / `..`; `-A` excludes those two names.
  `-d` lists directory operands themselves. `-F` adds type indicators and
  `-p` marks only directories. `--` ends option parsing.
- `-t` / `-S` sort newest / largest first, `-r` reverses, and `-U` skips
  sorting. `-f` includes all entries without sorting. `--dirs-first` groups
  directories. Explicit file operands are sorted together before directory
  sections, which are also sorted. `-c` / `-u` select status-change / access
  time for display and `-t`; `--time=mtime|ctime|atime` also works.
- `-i` / `-s` add inode / allocated-space prefixes without replacing an
  explicitly selected text format. `-h` uses readable sizes (the default),
  `--bytes` uses bytes, and `-k` uses KiB for allocated space and bytes for file
  sizes. `-n` shows numeric ownership, `-g` omits owner, and `-o` omits group.
- Names sort naturally (`photo2` before `photo10`), ignoring ASCII case.
- Colors use your terminal's palette; ordinary files keep its default
  foreground. `LS_COLORS` type keys (including `or`/`mi`) and `*suffix` rules
  apply, with a case-insensitive fallback. Dangling symlinks are red. `NO_COLOR`
  and `--color=auto|always|never` are honored.
- Icons: Nerd Font glyphs on all normal terminals, including multiplexers and
  SSH sessions. Font support is a terminal setting, not inferred from its name.
  Use `--icon-style=portable` for basic Unicode symbols if your font lacks Nerd
  Font glyphs, or `--no-icons` to hide icons. `--icons=auto|always|never` controls
  visibility separately; `always` also enables icons in pipes, while `auto`
  keeps pipes and `TERM=dumb` plain. Changing icon style alone never forces icons.
- Names are clickable (OSC 8) on local terminals; `--hyperlink=auto|always|never`.
- Pipes get complete plain names, one per line. Control characters and invalid
  UTF-8 are escaped everywhere; no name is shortened. Explicit `-q` replaces
  those characters with `?`; `-b` restores escaping.

## Links, recursion and metadata

`-H` follows symlinks supplied as operands; `-L` follows all links; `-P` keeps
the links themselves. The last policy wins. Without a policy, directory-link
operands are followed unless `-l`, `-d` or `-F` is active. Other links keep their
identity, and long listings show their targets. Dangling links remain visible;
explicitly requesting their target reports an error and continues the listing.

`-R` prints each directory and then its subdirectories in the selected order.
Hidden directories follow `-a`/`-A`; `.` and `..` are never descended into.
`-d` takes precedence over recursion. `-RL` follows directory links with
ancestor device/inode checks: loops report an error, and sibling directories
still print. Traversal is iterative and retains pending directory names, not
the whole tree or an open descriptor per depth. Preview attempts, image bytes
and placements share one budget across all operands and recursive sections.

Explicit long output probes extended metadata; automatic compact details and
plain names skip those extra queries. `@` marks extended attributes and `+`
marks ACLs (on macOS an attribute marker takes precedence). Optional details:

| Option | Information |
| --- | --- |
| `-@`, `--xattrs` | Extended attribute names and sizes, never their contents |
| `-e`, `--acl` | macOS allow/deny entries with UUID principals and inheritance; Linux POSIX access/default ACLs with numeric IDs |
| `-O`, `--flags` | macOS file flags such as `uchg`, `hidden` and `compressed`; `-` where unsupported |
| `-Z`, `--context` | Linux SELinux context; `-` where absent or unsupported |

These options select long output. `--fields=flags` also selects file flags.
Attribute name/value buffers are limited to 64 KiB, attributes to 256 entries,
each Linux ACL to 256 entries, and macOS ACLs to the platform's 128-entry limit.
Unavailable or oversized optional details are labeled; they never hide files.

The command intentionally keeps natural, ASCII-case-insensitive ordering,
readable sizes, local date formatting and safe text. `-U` means unsorted (GNU
style), `-G` enables terminal color (macOS style), and `-v` selects natural
ordering. This is practical command compatibility, not byte-for-byte output
compatibility or a raw filename protocol for scripts.

## Thumbnails

Images, files, folders and links share one ordering: a thumbnail represents its
entry, and a failed preview never hides a name.

- In a direct Ghostty or Kitty session, listings where at least half the entries
  are images, or that fit one row, become a thumbnail grid. Other listings show
  one-row miniatures beside names in the long view. Multiplexers, SSH sessions
  and unknown terminals use text unless `--protocol=kitty` forces graphics.
- Every tile has pixels: a thumbnail, or built-in folder, file, media, link,
  broken-link or error artwork.
- Formats: PNG, JPEG (with EXIF orientation), GIF (first frame), WebP, BMP, ICO
  and a restricted, self-contained SVG subset. On macOS, HEIC/HEIF, AVIF, TIFF,
  JPEG XL, Photoshop and camera RAW files also preview, using the system's
  ImageIO decoders (loaded only when previewing). Video frames are not decoded.
- Up to four decoders work ahead in parallel; output keeps listing order.
- `--thumbnail-size=1..12` sets grid height in rows (default 3).
  `--preview-limit=0..4096` sets attempts across all paths (default 256).
- Tall galleries print into scrollback. Once a budget is spent, remaining names
  continue as compact text.
- An opt-in thumbnail cache (`--cache-dir=PATH`) is bounded to 64 records and
  about 19 MiB; see [cache design](cache.md). `--diagnose PATH` explains the
  chosen layout, limits and decoders without decoding anything.

| Resource | Bound |
| --- | --- |
| Preview attempts | 256 by default, 4,096 at most; failures and cache hits count |
| Image commands | 128 MiB (counted uncompressed) and 4,096 placements per invocation |
| Decoding | Up to four sources at once, at most eight results ahead of output |
| Source file | 32 MiB for the Rust decoders; 256 MiB for macOS ImageIO, which reads only what it needs |
| Decoded image | 16 million pixels at full size, 64 MiB decoder allocations. JPEG, HEIF and AVIF decode at reduced resolution on macOS, up to 64× larger; bigger TIFF/PSD/JPEG XL/RAW use embedded previews |
| SVG | 256 KiB, 4,096 XML nodes, 32 levels; no text, external or embedded images, filters, masks, patterns, markers or `use` |
| Thumbnail | Grid at most 320×240 pixels; long view at most 96×64 |
| Cache | Opt-in; 64 records, under 20 MiB of file contents |

Exit codes: 0 success (including closed pipes), 1 listing, output or cache-clear
error, 2 invalid options.
