# Windows Development

Tested baseline: Windows 10 21H2+ / Windows 11, x64. Do not claim other configurations without testing.

## Required

1. **Visual Studio 2022 Build Tools** with the **Desktop development with C++** workload (MSVC linker + Windows SDK):

   ```powershell
   winget install --id Microsoft.VisualStudio.2022.BuildTools -e --override "--add Microsoft.VisualStudio.Workload.VCTools --includeRecommended"
   ```

2. **WebView2 Runtime** — preinstalled on Windows 11; on Windows 10 install via [Microsoft's page](https://developer.microsoft.com/microsoft-edge/webview2/) (Evergreen Bootstrapper).
3. Git, Node 22, pnpm, Rust — installed by `pwsh ./scripts/setup.ps1`.

## Not required

- Android tooling (no mobile targets yet).
- Admin rights beyond what winget itself requests.

## Verify

```powershell
pwsh ./scripts/doctor.ps1
cargo check --workspace
```

## Notes

- Long-path issues: if `cargo` complains about path length, enable long paths (`git config --global core.longpaths true` is usually sufficient for this repo).
- Antivirus exclusions are a personal choice; the build does not require disabling Defender and we never ask you to.
