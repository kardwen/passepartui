# passepartui

<img src="passepartui_screenshot.png" width="80%">

A TUI for pass

## Introduction

I started this project as a way to practice programming in Rust while reading the [Rust Book](https://doc.rust-lang.org/stable/book/title-page.html).
Therefore this project is still in an early alpha version, however user interaction is mostly finished.

`passepartui` relies for all decryption operations on [pass](https://www.passwordstore.org/), one-time passwords (OTP) are handled by [`pass-otp`](https://github.com/tadfisher/pass-otp).
Functionality for changing the password store is currently not implemented.
More on the current state of development can be found below.

The name `passepartui` is a combination of "passepartout", French for "master key", and "TUI".

## Features

* Easy navigation with arrow keys and Vim keybinding
* Searching and filtering of passwords
* Support for viewing and copying operations
  for passwords and one-time passwords
* Mouse support (limited)

## Installation

### Requirements

* Unix (tested on Linux so far)
* `pass`, optionally `pass-otp` for one-time passwords
* Rust and cargo

### Installation from crates.io

`passepartui` can be found on crates.io [here](https://crates.io/crates/passepartui). Note that the version must be specified.

```sh
cargo install --version 0.1.0-alpha passepartui
```

Type `passepartui` to run the app (provided that `~/.cargo/bin` has been added to `$PATH`).

### Manual installation

Clone the repository and change to it:

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

## Contrib

The `contrib` directory contains additional files, for now an example for a desktop file.

A desktop entry lets you start `passepartui` with your application menu. Edit the desktop file `passepartui.desktop` to use your terminal emulator for running `passepartui` and copy it to `$XDG_DATA_HOME/applications` which is usually `~/.local/share/applications`.

## Development

Contributions are welcome! For architectural changes please start with opening an issue.

Build with [Ratatui](https://github.com/ratatui/ratatui)

TODO:

* General refactoring
* Tests
* Mouse support overhaul
* Localisation for last modified column
* Button animations for keyboard shortcuts

Planned for future versions:

* Background updates
* Support for symbolic links in store
* Tree view for folders <https://github.com/EdJoPaTo/tui-rs-tree-widget>
* Decryption of password files with rust (possibly with sequoia-openpgp)
  This allows for
  * showing all passwords at once when scrolling in the corresponding view mode
  * displaying flags for set fields in password table
* Column sorting
* Theming

Clippy:

```sh
rustup component add clippy
```

```sh
cargo clippy
cargo fmt
```
