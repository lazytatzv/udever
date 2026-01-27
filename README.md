# 🦀 udever

[![Crates.io](https://img.shields.io/crates/v/udever.svg)](https://crates.io/crates/udever)
[![License](https://img.shields.io/crates/l/udever.svg)](https://github.com/lazytatzv/udever/blob/main/LICENSE)

> **Stop writing udev rules by hand.**
>
> `udever` is a blazing fast, interactive CLI tool to manage udev rules for your USB devices. Generate permission rules, create symlinks, and reload drivers without leaving your terminal.

---

## ⚡ Features

- **Interactive Selection**: Fuzzy-search your connected USB devices. No more `lsusb` grep hunting.
- **Smart OS Detection**: Automatically selects the correct group (`uucp` for Arch/Manjaro, `dialout` for Debian/Ubuntu).
- **Safe & Robust**:
  - Filters out Root Hubs to prevent system accidents.
  - Performs `systemd-udevd` health checks before running.
  - Validates syntax before writing.
- **Instant Feedback**: Automatically reloads rules and triggers device events (`udevadm trigger`).
- **Editor Integration**: Open generated rules in `nano`, `vim`, or `nvim` for manual tweaking.
- **Symlink Generator**: Easily create persistent device names (e.g., `/dev/my_arduino`).

## 🚀 Installation

### From Crates.io (Recommended)
You need [Rust](https://www.rust-lang.org/tools/install) installed.

```bash
cargo install udever
```

Since `udever` requires root privileges to manage udev rules, i recommend making it available to sudo:

```bash
# Ensure that cargo's path is valid
# in ~/.bashrc etc.
export PATH="$HOME/.cargo/bin:$PATH"
# You should link
sudo ln -s $HOME/.cargo/bin/udever /usr/local/bin/udever
```

### Arch Linux (AUR)
You can install `udever` from the AUR using an AUR helper like `yay` or `paru`.

```bash
yay -S udever
# or
paru -S udever
```

### From Source
```bash
git clone [https://github.com/lazytatzv/udever.git](https://github.com/lazytatzv/udever.git)
cd udever
cargo install --path .
```

## 📖 Usage

**Note: Root privileges are required to write into `/etc/udev/rules.d/`.**

Run the interactive wizard:
```bash
sudo udever
```

### Quick Commands

Create a rule for a specific device ID (VID:PID):
```bash
sudo udever --id 1234:5678
```

Generate shell completions (bash/zsh/fish):
```bash
udever --completion zsh > _udever
```

## 🎮 Workflow Demo

```text
$ sudo udever

? Select USB Device (Type to search)
> 1. STMicroelectronics [0483:3748] ST-LINK/V2
  2. FTDI [0403:6001] FT232R USB UART
  3. Logitech [046d:c52b] USB Receiver

? Permission
> Current user only (uaccess)
  Group 'uucp' (mode 0660)
  Everyone (mode 0666)

? Create symlink? [Y/n] Y
? Symlink Name: stlink_v2

--- Preview: /etc/udev/rules.d/99-stlink_v2.rules ---
SUBSYSTEM=="usb", ACTION=="add", ATTRS{idVendor}=="0483", ATTRS{idProduct}=="3748", TAG+="uaccess", SYMLINK+="stlink_v2"
-----------------------------------------------------

? Write to file? [Y/n] Y
File created.
Reloading udev rules...
Success: /dev/stlink_v2
```

## 🛠 Troubleshooting

**"udev daemon is NOT active"**
`udever` relies on `systemd-udevd`. If the tool warns you, try starting the service:
```bash
sudo systemctl start systemd-udevd
```

## 🤝 Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

1. Fork it
2. Create your feature branch (`git checkout -b feature/cool-feature`)
3. Commit your changes (`git commit -am 'Add some cool feature'`)
4. Push to the branch (`git push origin feature/cool-feature`)
5. Create a new Pull Request

## 📄 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.
