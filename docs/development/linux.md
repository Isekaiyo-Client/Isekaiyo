# Linux Development

Tested baselines: Debian/Ubuntu LTS, Fedora latest, Arch current — x86_64. Other distros: likely fine with equivalent packages; don't file "works" claims without testing.

## Compiler & core tools

All distros: recent GCC or Clang, make, pkg-config, curl, git.

## Tauri/WebKit dependencies (the part people miss)

Tauri uses the system WebKitGTK. You need the **4.1** API development headers:

### Debian / Ubuntu

```sh
sudo apt-get update
sudo apt-get install -y build-essential curl wget file pkg-config \
  libwebkit2gtk-4.1-dev libxdo-dev libssl-dev \
  libayatana-appindicator3-dev librsvg2-dev
```

### Fedora / RHEL-family

```sh
sudo dnf install -y gcc gcc-c++ make pkgconf-pkg-config curl file \
  webkit2gtk4.1-devel libxdo-devel openssl-devel \
  libappindicator-gtk3-devel librsvg2-devel
```

### Arch

```sh
sudo pacman -S --needed base-devel curl file pkgconf \
  webkit2gtk-4.1 libxdo openssl appmenu-gtk-module librsvg
```

## Packaging tools (only when working on packaging)

- AppImage: `linuxdeploy`, `appimagetool` (downloaded into CI, not system)
- Flatpak: `flatpak-builder` + `org.gnome.Platform//46` SDK (see `packaging/`)
- deb/rpm builds happen in CI; local `cargo deb`/`cargo generate-rpm` optional.

## Verify

```sh
sh ./scripts/doctor.sh
cargo check --workspace
```

Wayland note: if windows render oddly under Wayland, try `WINIT_UNIX_BACKEND=x11 cargo run -p ikk-launcher`.
