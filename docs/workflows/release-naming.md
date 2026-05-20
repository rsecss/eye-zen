# Release Asset Naming

## Template

```text
Eyezen_{version}_{arch}[-setup].{ext}
```

MSI may add `_en-US`; this is produced by Tauri.

## Assets

| Platform | Asset |
|----------|-------|
| Windows NSIS | `Eyezen_{ver}_x64-setup.exe` |
| Windows MSI | `Eyezen_{ver}_x64_en-US.msi` |
| Windows portable | `Eyezen_{ver}_x64-portable.zip` |
| macOS Apple Silicon | `Eyezen_{ver}_aarch64.dmg` |
| macOS Intel | `Eyezen_{ver}_x64.dmg` |
| Linux AppImage | `Eyezen_{ver}_amd64.AppImage` |
| Linux DEB | `Eyezen_{ver}_amd64.deb` |

## Rules

- Keep the product name from `src-tauri/tauri.conf.json`.
- Use semantic version without the `v` prefix.
- Follow Tauri's native architecture names unless release CI overrides them.
- Add new formats here before publishing them.
