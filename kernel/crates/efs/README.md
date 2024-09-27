[![crates.io-badge]][crates.io-link] [![license-badge]][license-link]

[crates.io-badge]: https://img.shields.io/crates/v/efs.svg
[crates.io-link]: https://crates.io/crates/efs

[license-badge]: https://img.shields.io/badge/License-GPL%20v3-blue.svg
[license-link]: http://www.gnu.org/licenses/gpl-3.0

# Extended fs

An OS and architecture independent implementation of some Unix filesystems in Rust.

> [!WARNING]
> This crate is provided as is and do not offer any guaranty. It is still in early
> development so bugs are excepted to occur. If you find one, please report it at
> <https://codeberg.org/RatCornu/efs/issues>. In all cases, please do **NOT** use
> this library for important data, and make sure to backup your data before using it.

## Features

* `no_std` support (enabled by default).

* General interface for UNIX filesystems.

* `read`/`write` operations on regular files, directories and symbolic links.

* Compatible with any device implementing `Read + Write + Seek`.

* Fully documented.

## Supported filesystems

* [`ext2`](https://en.wikipedia.org/wiki/Ext2): ✅

If you want more supported filesystems, do not hesitate to open an issue on <https://codeberg.org/RatCornu/efs/issues>.

## Usage

Add this to your `Cargo.toml`:

```
[dependencies]
efs = "0.4"
```

See examples on <https://docs.rs/efs> in [`src/lib.rs`](src/lib.rs).

## Features

* `ext2`: enable the `ext2` filesystem support

* `std`: enable the features depending on the standard library

By default, only the `ext2` feature is set.

## License

Licensed under the GNU General Public License v3.0 which can be found [here](LICENSE).
