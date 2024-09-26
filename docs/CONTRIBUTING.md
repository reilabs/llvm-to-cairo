# Contributing

This document exists as a brief introduction to how you can contribute to the LLVM to Cairo project.
It includes a guide to [getting started](#setting-up-for-development) and
[contributing to `main`](#getting-your-work-on-main).

This repository is written in [Rust](https://www.rust-lang.org), a high-performance and low-level
language with a smart compiler that helps to write reliable and fast software. If you haven't worked
with Rust before, take a look at the ["New to Rust?"](#new-to-rust) section below!

## Setting Up for Development

Unfortunately the complexity of having LLVM available for building this project means that we have
to be a bit careful with our environment setup. We recommend using [lix](https://lix.systems) to
work with the flake configuration included in the repository.

We assume that you are either running on linux or macOS at this stage.

1. Install [lix](https://lix.systems/install/) using the lix installer—you will want to say "yes"
   when asked if you want to enable flakes—and make sure that you can successfully run the `nix`
   command.

   ```shell
   curl -sSf -L https://install.lix.systems/lix | sh -s -- install
   # Follow the instructions
   nix --version
   ```

2. Clone the repository. If you don't want to contribute directly you can use HTTPS clones:

   ```shell
   git clone https://github.com/reilabs/llvm-to-cairo
   ```

   If you _do_ want to contribute directly to the tree, we recommend cloning over SSH:

   ```shell
   git clone git@github.com:reilabs/llvm-to-cairo.git
   ```

3. Enter the cloned directory (`cd llvm-to-cairo`) and launch a development shell using
   `nix develop`. This will launch into your default shell, but you can override this by running
   `nix develop ".#ci" --command <your-command>` to use the CI dev shell instead.

4. Within this shell you are able to run Cargo commands such as `cargo build`, which will build the
   project, or `cargo test` which will run the tests.

### Pre-Commit Hooks

In order to ensure uniform formatting we use a series of precommit hooks that run things like
linters and other checks before things get committed.

1. **Set Up:** Run `npm install`. This will install and set up the hooks that run before every
   commit.
2. **Test:** When you run `git commit`, you will see a progress indicator show up that gives insight
   into the linting and formatting processes that are running. If any of these fail, then

In order to run these hooks you will either need to be within the `nix develop` shell, which
provides all of these, or to have the following installed and available on your path: `nodejs`,
`npm` and `npx`, and both `rustfmt` and `clippy`.

### Setting Up Your IDE

Due to the complexities of getting some IDEs to work with nix-based Rust projects, we recommend
using a system-wide installation of the correct toolchain (as specified in
[`Cargo.toml`](../crates/compiler/Cargo.toml)). Then, we recommend launching your IDE from the
development shell provided by `nix develop`, as this will give it the correct environment variables
to find the LLVM install.

We cannot provide instructions for all IDEs out of the box, but please feel free to add instructions
for your own IDE or editor here if you are working on the project.

#### RustRover

For those using RustRover you will need to go to the top-level "Rust" section in settings, and point
both "Toolchain location" and "Standard library" to a **system-level install** of both. For example:

```text
Toolchain Location = /Users/<you>/.rustup/toolchains/stable-platform-here/bin
Standard library = /Users/<you>/.rustup/toolchains/stable-platform-here/lib/rustlib/src/rust
```

Specifically do not point to the locations provided by `nix` for the development shell, as RustRover
seems to have issues with these.

## Getting Your Work on `main`

For contributions this repository works on a
[Pull Request](https://github.com/reilabs/llvm-to-cairo/pulls) and subsequent review model,
supported by CI to check that things avoid being broken. The process works as follows:

1. If necessary, you fork the repository, but if you have access please create a branch.
2. You make your changes on that branch.
3. Pull-request that branch against `main`.
4. The pull request will be reviewed, and CI will run on it.
5. Once reviewers accept the code, and CI has passed, it will be merged to main!

## New to Rust?

If you are new to working with Rust, a great place to start is the official
[Rust Book](https://doc.rust-lang.org/book/). It gives a great overview of the language and general
style. It's also worth getting familiar with the following tools:

- [Rustup](https://rustup.rs), the Rust toolchain installer.
- [Cargo](https://doc.rust-lang.org/cargo/), the Rust build tool and package manager.
- [docs.rs](https://docs.rs), a site providing up-to-date crate (package) documentation for all
  packages published to [crates.io](https://crates.io), the official package registry.
- [Rustdoc](https://doc.rust-lang.org/rustdoc/index.html), the ecosystem's official documentation
  tool.

In terms of development tooling, there are two major options in this space.

- [Rust Analyzer](https://rust-analyzer.github.io) is the official implementation of the
  [Language Server Protocol](https://microsoft.github.io/language-server-protocol/) for Rust, and
  will work with any LSP compatible host.
- [RustRover](https://www.jetbrains.com/rust/) is the fully-featured JetBrains IDE for working with
  Rust in single- and multi-language projects. The Rust support plugin can also run in other
  JetBrains IDEs if needed.
