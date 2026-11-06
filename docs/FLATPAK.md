# Upstream Flatpak builds

Tadpole publishes its own Flatpak bundle as a GitHub Actions artifact for changes to `main` and `codex/**` branches and for pull requests. Future version-tag releases also include the bundle alongside the other packages. The CI artifact is retained for 14 days; release assets are retained with the release. This is an x86_64 build; other architectures have not been validated.

The upstream Flatpak manifest and build script were created with AI assistance. **They are for this project's GitHub distribution only and must not be copied or reused as a Flathub submission manifest.** Flathub's [AI policy](https://docs.flathub.org/docs/for-app-authors/requirements#generative-ai-policy) prohibits AI-generated or AI-assisted manifest content. Flathub submission remains a separate human-owned process; see [the readiness audit](FLATHUB_READINESS.md).

## Build locally

Install Flatpak, Flatpak Builder, Rust/Cargo, and add the Flathub remote in the user installation. The build script resolves the GNOME 51 SDK, runtime and corresponding Rust SDK extension as needed. A network connection is needed for that setup and for downloading crates in `Cargo.lock`; the actual application compilation runs offline in the Flatpak build sandbox using vendored sources.

```sh
flatpak remote-add --user --if-not-exists flathub https://dl.flathub.org/repo/flathub.flatpakrepo
scripts/build-flatpak.sh
flatpak install --user dist/tadpole.flatpak
flatpak run com.tadpole.seo
```

Pass an output filename as the first argument to `build-flatpak.sh` to put the bundle elsewhere. Build staging is created in a temporary directory and removed on exit. In a Git checkout, the script copies tracked and non-ignored files, excluding the Git database, ignored credentials, saved crawls, target directory and previous distribution artifacts. For source archives without Git, it applies explicit exclusions. Dependency versions and checksums come from `Cargo.lock`; the runtime branch is specified in the upstream manifest and must be maintained as GNOME releases evolve.

## Access and limitations

The app needs network access to crawl websites and to use an optional, explicitly requested AI provider. It uses Wayland or X11, graphics access and IPC for its desktop interface. File chooser access should be used for opening and saving projects; test the chosen export directory in your desktop environment. The bundle does not grant blanket access to the host home directory.

JavaScript rendering needs a compatible Chrome/Chromium executable **inside** the sandbox. A browser installed on the host is not automatically available. Disable JavaScript rendering unless an accessible browser has been deliberately packaged; the application reports this limitation when the browser is unavailable.

The upstream Flatpak retains the existing `com.tadpole.seo` application ID. If a native Tadpole instance is already running, close it before launching the Flatpak to avoid activating that existing application. Flatpak stores its configuration and data separately from the native installation.
