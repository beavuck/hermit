### Cargo

Manage your install from Cargo, Rust's package manager.
See [cargo.md](cargo.md).

### Docker / docker-compose

Skip installation and run Hermit in a container.
See [docker.md](docker.md).

### Raw binary

**Linux:**

```bash
ARCH=$(uname -m | sed 's/x86_64/amd64/;s/aarch64/arm64/') \
&& TAG=$(curl -fsSL "https://gitlab.com/api/v4/projects/80082599/releases/permalink/latest" \
  | grep -o '"tag_name":"[^"]*"' | cut -d'"' -f4) \
&& sudo curl -fsSL "https://gitlab.com/api/v4/projects/80082599/packages/generic/hermit/${TAG}/hermit-linux-${ARCH}" \
  -o /usr/local/bin/hermit \
&& sudo chmod +x /usr/local/bin/hermit
```

Re-run the same command to update to the latest release.

To uninstall, simply delete the binary:

```bash
sudo rm /usr/local/bin/hermit
```

**Windows:**

Download `hermit-windows-amd64.exe` from
the [latest release](https://gitlab.com/beavuck-services/hermit/-/releases/permalink/latest).
