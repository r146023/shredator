# Installation and Build

Shredator is intended to be compiled as a normal Rust command-line application.

## Requirements

- Rust toolchain with `cargo`.
- The `rand` crate in `Cargo.toml`.
- Permission to write to and delete the target files.

## Minimal `Cargo.toml`

```toml
[package]
name = "shredator"
version = "0.1.0"
edition = "2021"

[dependencies]
rand = "0.8"
```

If the source is compiled with Rust 2024 and `rand` emits warnings around `gen`, use the current `rand` APIs already used by the updated source: `rng.fill(...)` rather than `rng.gen(...)`.

## Build

```bash
cargo build
cargo build --release
```

The release binary will usually be located at:

```text
target/release/shredator
```

On Windows:

```text
target\release\shredator.exe
```

## Install locally

From the project directory:

```bash
cargo install --path .
```

Or copy the release binary somewhere on your `PATH`:

```bash
cp target/release/shredator ~/.local/bin/shredator
```

On Windows, copy `shredator.exe` into a tools directory that is on `PATH`, for example:

```powershell
Copy-Item .\target\release\shredator.exe C:\Tools\shredator.exe
```

## Check the binary

```bash
shredator --help
```

The help output should include:

- `--passes`
- `--pattern`
- `--max-depth`
- `--include`
- `--exclude`
- `--benchmark`
- `--zero-names`
- `--file-list`
- `--output`
- `--json`
- `--jsonl`
- `--machine-readable`

## Recommended binary naming

For standalone CLI use, `shredator` is fine.

For embedding inside another project, use a stable predictable name:

```text
bin/shredator.exe
bin/shredator
vendor/shredator/shredator.exe
```

Wrapper code should resolve the binary path explicitly rather than assuming the user's global `PATH` contains the right version.

## Versioning recommendation

Because the machine-readable schema is wrapper-facing, version both the binary and the schema. The updated source emits:

```text
shredator.machine.v1
```

Do not change the meaning of existing fields inside `shredator.machine.v1`. Add fields freely, but avoid renaming or removing fields unless the schema version changes.
