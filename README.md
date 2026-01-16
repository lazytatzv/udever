# udever

**The Zen udev rule generator for Arch Linux.** Simple, interactive, and safe. Written in Rust.

`udever` creates udev rules for USB devices without the hassle of manually looking up IDs or reloading drivers. It prioritizes security (`uaccess`) and follows Arch Linux standards (`uucp` group).

## Features

* **¿ Fuzzy Search**: Instantly find your device from `lsusb` by typing a few characters.
* **¿¿ Secure Defaults**: Uses `TAG+="uaccess"` (systemd-logind) instead of insecure `MODE="0666"`.
* **¿ Arch Native**: Correctly handles `uucp` group for serial devices (Arduino, ESP32, etc.).
* **¿ Safety First**: Previews the generated rule before writing to `/etc/udev/rules.d/`.
* **¿ Auto-Reload**: Automatically runs `udevadm control --reload` and `trigger` upon success.
* **¿ Shell Completion**: Supports Bash, Zsh, and Fish completions out of the box.

## Installation

### AUR (Recommended)
You can install `udever` from the AUR:

```bash
yay -S udever-git
```

### Cargo
If you have Rust installed:

```bash
cargo install udever
```

## Usage

Run as root (required to write to `/etc/`):

```bash
sudo udever
```

### Interactive Mode
1.  **Select Device**: Type to search for your USB device (Fuzzy match).
2.  **Symlink (Optional)**: Create a persistent `/dev/my_device` link.
3.  **Permission**: Choose from:
    * `uaccess`: (Recommended) Only the currently logged-in user can access the device.
    * `Everyone`: Sets `MODE="0666"` (Insecure, useful for debugging).
    * `Group 'uucp'`: Sets `GROUP="uucp", MODE="0660"` (Standard for serial devices).
    * `Open in editor`: Open the rule in `$EDITOR` (vim/nano) for advanced configuration.

### CLI Mode (Scripting)
You can skip the device selection by providing the Vendor:Product ID directly:

```bash
sudo udever --id 1234:5678
```

## Management
`udever` also acts as a rule manager.
* **Edit**: Select an existing rule to open in your editor.
* **Delete**: Remove old rules and automatically reload udev.

## Why "uaccess"?
Old tutorials suggest `MODE="0666"` (everyone can read/write) or adding users to groups (`plugdev`).
Modern Linux systems use **Access Control Lists (ACLs)** via `systemd-logind`.

`udever` applies `TAG+="uaccess"`, which dynamically grants permission *only* to the user currently sitting at the physical terminal. It is the most secure and modern way to handle USB permissions.

## License
MIT
