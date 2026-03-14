### Quick install

```bash
curl -fsSL https://gitlab.com/beavuck-services/hermit/-/raw/main/scripts/install.sh | sh
```

### Manual install

```bash
TAG=$(curl -fsSL "https://gitlab.com/api/v4/projects/80082599/releases/permalink/latest" \
  | grep -o '"tag_name":"[^"]*"' | cut -d'"' -f4) \
&& sudo curl -fsSL "https://gitlab.com/api/v4/projects/80082599/packages/generic/hermit/${TAG}/hermit" \
  -o /usr/local/bin/hermit \
&& sudo chmod +x /usr/local/bin/hermit
```

Re-run the same command to update to the latest release.