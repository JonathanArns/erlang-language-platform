# Building statically linked ELP

You can build a statically linked ELP using the [cross](https://github.com/cross-rs/cross) tool
and a little bit of setup.
Rust comes with pretty good cross compilation features out of the box, but 
it can still require a lot of setup, since building against e.g. musl instead of libc, requires
a copy of musl available on your system etc.
Cross makes that as simple as possible by providing a docker image with all of those dependencies
for pretty much every build target.

If that was all we needed, we would be done now, but ELP has some other build dependencies, like an OTP,
so we need to make those available in cross's container at build time.
For that we have a [dockerfile](./cross.Dockerfile) and a cross config [file](./Cross.toml) pointing at that
dockerfile.

Since we also need a built eqwalizer available, our dockerfile assumes that that is in its build context
and just copies it from there.

With all those prerequisites in place we can run
```
cross build --target x86_64-unknown-linux-musl
```
to build a statically linked elp for linux.
This binary will of course still need a JVM to run eqwalizer, but it will not depend on glibc, instead
linking statically against musl.

# Erlang Language Platform (ELP)

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="./logo/elp_final_Full_Logo_White_Text.png">
  <img alt="ELP logo" src="./logo/elp_final_Full_Logo_Color.png" width="100%">
</picture>

## Description

Designed at **WhatsApp** and inspired by the success of the
[Rust Analyzer](https://rust-analyzer.github.io/) project, ELP provides **a
scalable, fully incremental, IDE-first library for the semantic analysis of
Erlang code**.

ELP includes a fully fledged **LSP language server for the Erlang programming
language**, providing advanced features such as go-to-definition, find
references, call hierarchy and more for your IDE of choice.

ELP is easily **extensible** and provides a convenient **API to implement
linters and refactoring tools for Erlang**.

## Terms of Use

You are free to copy, modify, and distribute ELP with attribution under the
terms of the Apache-2.0 and MIT licences. See [LICENCE-APACHE](./LICENCE-APACHE)
and [LICENCE-MIT](./LICENSE-MIT) for details.

## Get Started

Please refer to the
[official documentation](https://whatsapp.github.io/erlang-language-platform/docs/get-started/)
to get started on your favourite text editor and to learn how to configure your
projects to use ELP.

## References

- [rust-analyzer](https://github.com/rust-lang/rust-analyzer)

## Contributing

- [CONTRIBUTING.md](./CONTRIBUTING.md): Provides an overview of how to
  contribute changes to ELP (e.g., diffs, testing, etc)

## FAQ

Please refer to [the FAQ document](./FAQ.md) for answers to some common
questions, including:

- What's the difference between ELP and Erlang LS?
- Why not extend Erlang LS, rather than creating a new tool?
- Why is ELP implemented in Rust, rather than Erlang?

## License

erlang-language-platform is dual-licensed

- [Apache](./LICENSE-APACHE).
- [MIT](./LICENSE-MIT).
