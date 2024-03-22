# KerrBHy

A Kerr Black Hole simulator

# MSRV

rustc v1.77

# Building

Make sure you have cargo installed and have updated rustc to at least the MSRV.

```sh
cargo build --release
```

# Binaries

There are two binaries in this project:
* `kerrbhy`
* `sim`

`kerrbhy` is useful for creating images of blackholes given a config.
`sim` is useful for live demonstrations and saving configs to disk.

They can be ran by:

```sh
cargo run --release --bin $binary
```

# Git Notes

This repo contains submodules.
```sh
git clone --recurse-submodules
```
Will clone all submodules correctly.