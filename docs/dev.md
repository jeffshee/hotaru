# Development Note

Hotaru can be built using two methods: "Build and Install" and "Build as Flatpak."

1. **Build and Install**
   - This method requires you to have dependencies installed on your system.
   - It installs all components, including the binary, GSettings schema, icons, and a desktop file, into a prefix (`~/.local` by default).
   - If your distro has the necessary packages, "Build and Install" is the quickest Hotaru setup.

2. **Build as Flatpak**
   - This method doesn't require you to have dependencies installed on your system.
   - Instead, it downloads all the dependencies and builds them into a container.
   - The initial build for Flatpak may take longer, as it needs to build all the required dependencies.

> Common dev tasks (running, testing, linting, building, and Flatpak) are wrapped in the top-level `Makefile`. Run `make help` for the list.

## Submodules

The core renderer builds **without** any submodule. Two submodules are only
needed for specific paths — initialize them with:

```bash
git submodule update --init --recursive
```

- [`third_party/linux-wallpaperengine`](../third_party/linux-wallpaperengine) —
  the Wallpaper Engine **scene** backend, pinned to a commit of our
  [fork](https://github.com/jeffshee/linux-wallpaperengine). Built by
  `make wpe-lib` (which initializes it for you) and by the Flatpak.
- [`pkgs/flatpak/shared-modules`](../pkgs/flatpak/shared-modules) — Flathub's
  shared build modules (glu, glew), used only by the Flatpak.

Both must be initialized before `make flatpak`.

## Build and Install

### Dependencies

#### Rust

Install the Rust toolchain (`cargo`, `rustc`), preferably via [rustup](https://rustup.rs/).

#### Build dependencies

- Fedora:
```bash
sudo dnf install git meson gtk4-devel gstreamer1-devel gstreamer1-plugins-base-devel \
    webkitgtk6.0-devel gtk4-layer-shell-devel mpv-libs-devel
```

- Ubuntu:
```bash
sudo apt install git meson libgtk-4-dev libgstreamer1.0-dev libgstreamer-plugins-base1.0-dev \
    libwebkitgtk-6.0-dev libgtk4-layer-shell-dev libmpv-dev
```

Minimum versions: GTK 4.14, GStreamer 1.24, libmpv 2.x (mpv ≥ 0.35).

To build **without libmpv** (the mpv renderer is a default cargo feature), pass
`MESON_FLAGS="-Dmpv=false"` to `make build`, or use
`cargo build --no-default-features --features base`.

#### Scene backend (optional)

Wallpaper Engine **scene** wallpapers (`wallpaper_type: scene`) are rendered by
the bundled [linux-wallpaperengine](https://github.com/jeffshee/linux-wallpaperengine)
fork, which hotaru `dlopen`s at runtime — so the core build does not depend on
it. To build it locally:

```bash
make wpe-lib   # CEF-free; WPE_JOBS=N caps parallelism (default ~1 job / 2 GB RAM)
```

It additionally needs CMake, Ninja, and the following dev headers (Fedora
package names): `glm-devel glfw-devel glew-devel mesa-libGLU-devel
sdl2-compat-devel lz4-devel freetype-devel` plus the X11/Wayland/DBus dev
packages.
Point hotaru at the result with `HOTARU_WPE_LIBRARY` (see the SceneWidget
section of [renderers.md](renderers.md)). The Flatpak bundles this backend, so
no manual step is needed there.

#### Runtime dependencies

Video decoding goes through GStreamer (`gst-gtk4` renderer) or libmpv/FFmpeg
(`mpv` renderer, default) — install the usual codec/plugin packages for your
distro (e.g. `gstreamer1-plugins-good`, VA-API drivers) as needed. Hardware
decoding with the mpv renderer works out of the box where FFmpeg supports it
(`hwdec=auto-safe`).

#### Linting and formatting

```bash
make lint     # cargo clippy
make format   # cargo fmt
make test     # cargo test
```

### Install

`make install` installs into `~/.local` (no sudo). For a system-wide install, pass a prefix:
```bash
make install                      # ~/.local
sudo make install PREFIX=/usr/local
```

### Run

Hotaru **requires its GSettings schema to be installed** (it aborts on startup
otherwise), so run `make install` once before the first run. After that:

```bash
make run                                          # cargo run (debug build)
hotaru --config examples/config/wallpaper_per_monitor.json   # installed binary
```

Example configs live in [`examples/config/`](../examples/config/); edit the
monitor connector names and file paths to match your setup. Renderer and
playback settings are GSettings keys, e.g.:

```bash
gsettings set io.github.jeffshee.Hotaru video-renderer mpv   # or gst-gtk4
gsettings set io.github.jeffshee.Hotaru content-fit 2        # 0 fill, 1 contain, 2 cover
```

See [architecture.md](architecture.md) and [renderers.md](renderers.md) for
how it all fits together.

### Uninstall

```bash
make uninstall
```
This removes exactly what `make install` installed (from the same `PREFIX`).

## Build as Flatpak

First, please make sure you have `flatpak` and `flatpak-builder` installed on
your system. For more details, please refer to the
[Flatpak official documentation](https://docs.flatpak.org/en/latest/first-build.html).

All Flatpak packaging files live under [`pkgs/flatpak`](../pkgs/flatpak/),
including the manifest `io.github.jeffshee.Hotaru.json` and the bundled build
modules: gtk4-layer-shell; libmpv with its FFmpeg/libass/libplacebo
dependencies; the scene backend (`linux-wallpaperengine.json`, built CEF-free)
with its `glm`/`glfw` deps; and `glu`/`glew` pulled from the
[`shared-modules`](../pkgs/flatpak/shared-modules) submodule. GStreamer and
WebKitGTK come from the GNOME runtime.

> Remember to initialize the submodules (see [Submodules](#submodules)) before
> building the Flatpak.

### Environment Setup

For Flatpak development, VSCode with the Flatpak extension
(`bilelmoussaoui.flatpak-vscode`) is recommended. Alternatively, GNOME Builder
is also useful when building Flatpak applications.

### Build and Run

With the extension installed, press <kbd>F1</kbd> (command palette), search
for "flatpak", and run the desired action.

Alternatively, build and install it from the command line (run from the
repository root):
```bash
make flatpak
make flatpak-run
```
