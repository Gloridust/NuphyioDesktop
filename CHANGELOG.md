# Changelog

## [0.1.0] - 2026-11-07

First release. Linux only (x86_64).

### Desktop app

- Remap keys on NuPhy keyboards over a USB cable or the 2.4G receiver, fully offline.
- Click a key on the keyboard picture, then pick its new function. Every change is written right away, read back to confirm,
  and the whole key map is backed up first.
- Per-key reset to the factory setting, mode switching (for example M1 Mac / M2 Windows), and a backup list to restore from.
- Looks and works like the NuPhyIO web app. Interface in English and Simplified Chinese.

### Command line tool `nuphy`

- `list` / `info` / `keys` / `get` to inspect, `set` / `reset` to remap, `backup` / `restore` for the whole key map.

### Files

- `nuphy-desktop_0.1.0_amd64.deb`: for Ubuntu / Debian. Also installs the udev rule, so the keyboard opens without extra setup.
- `nuphy-desktop_0.1.0_amd64.AppImage`: for other distributions. Install the udev rule once (see the README).
- `nuphy-cli-0.1.0-x86_64-linux.tar.gz`: the `nuphy` command plus the udev rule.

### Known limits

- Only the mechanical keyboard family (Node75, Air75 V3, ...) is supported; Hall-effect (HE) models are not.
- Tested on a Node 75 LP (firmware 1.0.10.6). Other models should work but have not been tried.
- Advanced keys, lighting, macros and mode settings are not done yet.
- Bluetooth cannot be used for configuration (the same as the web app).
- No Windows or macOS build yet.
