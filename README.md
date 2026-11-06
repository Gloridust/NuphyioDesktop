# NuphyioDesktop
Nuphyio for Desktop | Linux/Windows/macOS support

NuPhy 官方配置工具 [NuPhyIO](https://drive.nuphyio.com) 是纯前端的 WebHID 应用，Linux 上常因 hidraw 权限连不上。
本项目把它的协议逆向出来，用 Rust 重新实现，目标是做一个原生桌面版。

## 现状

- `crates/nuphy-proto`：协议核心（帧编解码、握手、会话、设备表、键位解析），无 I/O，已有单元测试。
- `crates/nuphy-hid`：Linux hidraw 传输（通过 sysfs 枚举，无需 libudev）。
- `crates/nuphy-cli`：命令行工具 `nuphy`，目前只读：`list` / `info` / `keys` / `raw`。
- 只实现了机械款协议（Node75、Air75 V3 等）；Hall（HE）款只有命令表。
- **未在真机上验证过**，详见 [docs/PROTOCOL.md](docs/PROTOCOL.md) §9。

## 使用

```bash
# 1. 让当前用户能打开 hidraw（一次性）
sudo cp contrib/udev/70-nuphy.rules /etc/udev/rules.d/
sudo udevadm control --reload && sudo udevadm trigger --subsystem-match=hidraw
# 重新插拔键盘

# 2. 运行
cargo run -p nuphy-cli -- list
cargo run -p nuphy-cli -- info
cargo run -p nuphy-cli -- keys
cargo test --workspace
```

## 下一步

真机只读验证 → 解码 GetBase / 键位矩阵布局 → 写入命令（改键）→ GUI（候选：Tauri）→ Windows/macOS 传输（hidapi）。
