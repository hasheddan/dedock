# dedock

> NOTICE: `dedock` is highly experimental and should not be used in production
> environments. Expect rapid iteration and breaking changes.

`dedock` is a ~container~ runtime, with a particular focus on enabling embedded
software development across all platforms. It supports native "containers" on
both Linux and macOS.

## How It Works

`dedock` is not a container runtime in that it does not provide meaningful
isolation from a security perspective and is not compliant with the [OCI Runtime
Specification](https://github.com/opencontainers/runtime-spec). The primary
purpose of `dedock` is to enable the distribution of portable development
environments. It does so by partially adopting [OCI
images](https://github.com/opencontainers/image-spec) to distribute filesystem
bundles with tooling and dependencies pre-installed.

`dedock` uses [`chroot(1)`](https://linux.die.net/man/1/chroot) on Linux and
macOS to isolate the filesystem of the "containers" it runs. Because no other
isolation is employed, executables in the filesystem run natively on the host
machine, meaning that there is no virtualization layer, even when running on
macOS. As such, separate images must be built for macOS (Darwin) and Linux.

## Status

`dedock` is very much in the technical demo stage and should not be relied upon
for critical operations. The initial motivation for the project was to allow for
developers to build, flash, and debug software on embedded devices attached to a
host machine. As such, there are a number of defaults that would not make sense
for general usage, including always running with `stdout` / `stdin` / `stdout`
attached to a pseudoterminal and always mounting `/dev`.

The future of the project is very much dependant on feedback from community
members, but the following goals are presently in scope for `dedock`:

- Providing more options for configuration.
- Supporting rootless containers.
- Running on Windows.
- Maintaining a set of useful base images.
- Offering image build tooling.

## License

Licensed under either of

 * Apache License, Version 2.0
   ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license
   ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

## Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.
