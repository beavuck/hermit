# Installing and using Hermit via Cargo

## Install

```sh
cargo install beavuck-hermit
```

> 🔶 This builds and installs the `hermit` binary to `~/.cargo/bin/`. Make sure that directory is on your `PATH`.

To update to a newer version, run the same command again.

## Quick start

```sh
hermit --specs-dir path/to/your-specs-dir
```

## Loading specs

### A whole directory

```sh
hermit --specs-dir ./specs
```

### A single file

```sh
hermit --specs api.openapi.yml
```

### Multiple files

```sh
hermit --specs users.openapi.yml orders.openapi.yml
```

## Options and environment variables

See [options.md](options.md).
