# passepartui

[![crates.io](https://img.shields.io/crates/v/passepartui.svg)](https://crates.io/crates/passepartui)
[![Packaging status](https://repology.org/badge/tiny-repos/passepartui.svg)](https://repology.org/project/passepartui/versions)

<img src="passepartui_screenshot.png" width="75%">

A TUI for pass

## Introduction

`passepartui` is a text-based user interface (TUI) for [`pass`](https://www.passwordstore.org/) designed for fast and intuitive access to the password store.

**Features**:

* Easy navigation with arrow keys and Vim keybindings
* Searching and filtering of passwords
* Support for viewing and copying of
  passwords and one-time passwords
* Mouse support (limited)

This project is still in an *alpha* state, however, user interaction is mostly done. The reason for this is that I started this project as a way to practice programming in Rust while reading the [Rust Book](https://doc.rust-lang.org/stable/book/title-page.html).

Currently, no functionality for manipulating the password store (e.g. adding or deleting a password) is implemented. For those operations use `pass` directly from your terminal (refer to `man pass`).

The name `passepartui` is a combination of "passepartout", French for "master key", and "TUI".

## Installation

### Prerequisites

* Unix (tested on Linux so far)
* C system library [`gpgme`](https://gnupg.org/software/gpgme/index.html) for decryption operations
* Rust and cargo (when compiling from source)

### Installation from crates.io

`passepartui` can be found on [crates.io](https://crates.io/crates/passepartui).

```sh
cargo install passepartui --locked
```

Type `passepartui` to run the app (provided that `~/.cargo/bin` has been added to `$PATH`).

### Installation from the AUR

`passepartui` is available in the [AUR](https://aur.archlinux.org/packages/passepartui). You can install it with your favorite [AUR helper](https://wiki.archlinux.org/title/AUR_helpers), e.g.:

```sh
paru -S passepartui
```

### Installation from nixpkgs

`passepartui` is available in [nixpkgs](https://github.com/NixOS/nixpkgs). You
can install it in your system via the usual ways, or try it with:

```sh
nix run nixpkgs#passepartui
```

### Installation with Homebrew

A tap for installing `passepartui` with [Homebrew](https://brew.sh/) is available with this [repository](https://github.com/kardwen/homebrew-passepartui).

```sh
brew tap kardwen/passepartui
brew install passepartui
```

### Installation from Gentoo overlay

An [ebuild](https://gpo.zugaina.org/app-admin/passepartui) for installing `passepartui` is available via `lamdness` overlay:

```sh
eselect repository enable lamdness
emaint -r lamdness sync
emerge -av app-admin/passepartui
```

### Manual installation

Clone the repository and change to the directory:

```sh
git clone git@github.com:kardwen/passepartui.git
cd passepartui
```

Build and copy the executable to an appropriate location:

```sh
cargo build --release
cp target/release/passepartui ~/.cargo/bin
```

Run `passepartui` in a terminal.

## Miscellaneous

The `contrib` directory contains additional files, for now an example for a desktop entry file.

A desktop entry lets you start `passepartui` from an application menu in a new terminal window. Configure your preferred terminal emulator for running `passepartui` in the desktop file `passepartui.desktop` and copy it to `$XDG_DATA_HOME/applications` which is usually `~/.local/share/applications`.

## Development

Contributions are very welcome!
Take a look at the [open issues](https://github.com/kardwen/passepartui/issues) to get started.

List of libraries used (among others):

* [Ratatui](https://github.com/ratatui/ratatui) for the creation of the TUI
* [passepartout](https://github.com/kardwen/passepartout), which uses
  * [gpgme](https://github.com/gpg-rs/gpgme) for decryption of password files with [GnuPG](https://www.gnupg.org/)
  * [totp-rs](https://github.com/constantoine/totp-rs) for creation of one-time passwords (OTP)
