<div align="center">

# cargo-generate-minijinja

<img src="https://github.com/cargo-generate/cargo-generate/raw/52be603bab5329b0ba90a19cafd58973f8781fa7/resources/logo.png" width="256">


[![Build status](https://github.com/shellixyz/cargo-generate/workflows/Build/badge.svg)](https://github.com/shellixyz/cargo-generate/actions?query=workflow%3ABuild+branch%3Amain+)
[![crates.io](https://img.shields.io/crates/v/cargo-generate-minijinja.svg)](https://crates.io/crates/cargo-generate-minijinja)

</div>

> cargo, make me a project - Minijinja fork

This is a fork of `cargo-generate` that uses the **Minijinja** template engine instead of Liquid.
`cargo-generate-minijinja` is a developer tool to help you get up and running quickly with a new Rust
project by leveraging a pre-existing git repository as a template.

The main difference is that by default it is using the Minijinja engine with the trim_blocks and lstrip_blocks options turned on which means the lines with "{% %}" tags in files are removed and there is no need to deal with the Liquid style whitespace control to not have double white lines in the output, preserve indentation and stuff.

Example template which needs this fork to be rendered: [shellixyz/embassy-stm32-template](https://github.com/shellixyz/embassy-stm32-template)

## Quickstart

### Installation

```sh
cargo install cargo-generate-minijinja
```

### Usage

```sh
# templates on github
cargo generate-mj --git https://github.com/username-on-github/mytemplate.git

# or just
cargo generate-mj username-on-github/mytemplate

# templates on other git platforms
cargo generate-mj gl:username-on-gitlab/mytemplate # translates to https://gitlab.com/username-on-gitlab/mytemplate.git
cargo generate-mj bb:username-on-bitbucket/mytemplate # translates to https://bitbucket.org/username-on-bitbucket/mytemplate.git
cargo generate-mj sr:username-on-sourcehut/mytemplate # translates to https://git.sr.ht/~username-on-sourcehut/mytemplate (note the tilde)

# this scheme is also available for github
cargo generate-mj gh:username-on-github/mytemplate # translates to https://github.com/username-on-github/mytemplate.git

# for a complete list of arguments and options
cargo generate-mj --help
```

## License

Licensed under either of

* Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE)
  or [apache.org/licenses/LICENSE-2.0](https://www.apache.org/licenses/LICENSE-2.0))
* MIT license ([LICENSE-MIT](LICENSE-MIT) or [opensource.org/licenses/MIT](https://opensource.org/licenses/MIT))

at your option.

## About This Fork

This is a community fork of the original [cargo-generate](https://github.com/cargo-generate/cargo-generate) project. 
The primary difference is that this fork uses **Minijinja** as the template engine instead of Liquid, offering better 
Jinja2 compatibility and improved template functionality. The binary name `cargo-generate-mj` is invoked as `cargo generate-mj` 
(a cargo subcommand) and distinguishes this variant from the original `cargo generate` command while maintaining compatibility with existing templates.

### Contributions

Unless you explicitly state otherwise, any contribution intentionally
submitted for inclusion in the work by you, as defined in the Apache-2.0
license, shall be dual licensed as above, without any additional terms or
conditions.
If you want to contribute to `cargo-generate`, please read our [CONTRIBUTING notes].

cargo-generate would not be what it is today without the wonderful contributions from the community. Thank
you!

<a href="https://github.com/cargo-generate/cargo-generate/graphs/contributors">
  <img src="https://contrib.rocks/image?repo=cargo-generate/cargo-generate" />
</a>

[CONTRIBUTING notes]: CONTRIBUTING.md
