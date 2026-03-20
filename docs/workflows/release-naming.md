# Release Asset Naming Convention

## Naming Template

```
Eyezen_{version}_{arch}[-setup].{ext}
```

- **Eyezen**: Product name (from `tauri.conf.json` productName)
- **{version}**: Semantic version without `v` prefix (e.g., `0.1.0`)
- **{arch}**: CPU architecture identifier (see table below)
- **[-setup]**: Only for NSIS Windows installer
- **{ext}**: Platform-specific file extension

## Architecture Identifiers

| Architecture | Windows | macOS | Linux (DEB) | Linux (AppImage/RPM) |
|-------------|---------|-------|-------------|---------------------|
| 64-bit Intel/AMD | `x64` | `x64` | `amd64` | `amd64` |
| 64-bit ARM | `arm64` | `aarch64` | `arm64` | `aarch64` |

Note: Architecture identifiers follow Tauri bundler defaults, which align with each platform's native convention (Windows/macOS use `x64`, Debian uses `amd64`, etc.).

## Complete Asset Matrix

### Windows

| Format | Filename | Description |
|--------|----------|-------------|
| NSIS Installer | `Eyezen_{ver}_x64-setup.exe` | Recommended. Guided install wizard |
| MSI Installer | `Eyezen_{ver}_x64_en-US.msi` | Enterprise/silent deployment |
| Portable | `Eyezen_{ver}_x64-portable.zip` | No install needed, unzip and run |

### macOS

| Format | Filename | Description |
|--------|----------|-------------|
| DMG (Apple Silicon) | `Eyezen_{ver}_aarch64.dmg` | M1/M2/M3/M4 Macs |
| DMG (Intel) | `Eyezen_{ver}_x64.dmg` | Older Intel Macs |

### Linux

| Format | Filename | Description |
|--------|----------|-------------|
| AppImage | `Eyezen_{ver}_amd64.AppImage` | Portable, runs on any distro |
| DEB | `Eyezen_{ver}_amd64.deb` | Ubuntu / Debian |

## Example: v0.1.0 Release

```
Eyezen_0.1.0_x64-setup.exe          # Windows installer
Eyezen_0.1.0_x64_en-US.msi          # Windows MSI
Eyezen_0.1.0_x64-portable.zip       # Windows portable
Eyezen_0.1.0_aarch64.dmg            # macOS ARM
Eyezen_0.1.0_x64.dmg                # macOS Intel
Eyezen_0.1.0_amd64.AppImage         # Linux portable
Eyezen_0.1.0_amd64.deb              # Linux Debian/Ubuntu
```

## Reference Projects

| Project | Stars | Framework | Naming Style |
|---------|-------|-----------|-------------|
| RustDesk | 85k+ | Rust/Sciter+Flutter | `rustdesk-{ver}-{arch}.{ext}` |
| Clash-Verge | 45k+ | Tauri | `Clash.Verge_{ver}_{arch}[-setup].{ext}` |
| Blink Eye | — | Tauri | `Blink.Eye_{ver}_{arch}[-setup].{ext}` |
| VS Code | 170k+ | Electron | `VSCode{Type}-{arch}-{ver}.{ext}` |
| Obsidian | — | Electron | `obsidian-{ver}[-{arch}].{ext}` |

## Notes

- Tauri bundler auto-generates MSI, NSIS, DMG, AppImage, and DEB with the `{Name}_{ver}_{arch}` pattern
- Portable (Windows) requires manual zip step in CI — see `release.yml`
- RPM support can be added later if demand arises
- ARM64 Windows build can be added when Tauri runner support improves
