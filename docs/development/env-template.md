# Environment Template

> The repo does not commit an `.env.example` file directly (platform policy);
> copy this block into `.env.local` (git-ignored) if you need local overrides.
> All values are **non-secret** configuration only.

```sh
# Isolated dev data directory — never point at a real Minecraft install.
IKK_DEV_DATA_DIR=./IsekaiyoDev

# Log verbosity: error | warn | info | debug | trace
RUST_LOG=info
```

Rules (docs/security.md):

1. Never put real secrets in any committed file.
2. CI/production secrets live in GitHub repository settings or the OS keyring.
3. Tokens never enter logs; the diagnostics redaction layer enforces this.
