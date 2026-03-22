### Quick install (Linux)

```bash
curl -fsSL https://gitlab.com/beavuck-services/hermit/-/raw/main/scripts/install.sh | sh
```

### Manual install (Linux)

```bash
ARCH=$(uname -m | sed 's/x86_64/amd64/;s/aarch64/arm64/') \
&& TAG=$(curl -fsSL "https://gitlab.com/api/v4/projects/80082599/releases/permalink/latest" \
  | grep -o '"tag_name":"[^"]*"' | cut -d'"' -f4) \
&& sudo curl -fsSL "https://gitlab.com/api/v4/projects/80082599/packages/generic/hermit/${TAG}/hermit-linux-${ARCH}" \
  -o /usr/local/bin/hermit \
&& sudo chmod +x /usr/local/bin/hermit
```

Re-run the same command to update to the latest release.

### Windows

Download `hermit-windows-amd64.exe` from
the [latest release](https://gitlab.com/beavuck-services/hermit/-/releases/permalink/latest).

Alternatively, use Docker -- see [DOCKER.md](../DOCKER.md).
