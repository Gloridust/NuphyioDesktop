# NuphyioDesktop
Nuphyio for Desktop | Linux/Windows/macOS support

NuPhy 官方配置工具 [NuPhyIO](https://drive.nuphyio.com) 是纯前端的 WebHID 应用，Linux 上常因 hidraw 权限连不上。
本项目把它的协议逆向出来，用 Rust 重新实现，目标是做一个原生桌面版。

## 现状

- `crates/nuphy-proto`：协议核心（帧编解码、握手、会话、设备表、键位解析），无 I/O，已有单元测试。
- `crates/nuphy-hid`：Linux hidraw 传输（通过 sysfs 枚举，无需 libudev）。
- `crates/nuphy-cli`：命令行工具 `nuphy`：查看（`list` / `info` / `keys` / `get`）、改键（`set` / `reset`）、
  整表备份与还原（`backup` / `restore`）。完全离线，不需要浏览器或网络。
- 只实现了机械款协议（Node75、Air75 V3 等）；Hall（HE）款只有命令表。
- 在 Node 75 LP（固件 1.0.10.6）上验证过读写，其他型号未测，详见 [docs/PROTOCOL.md](docs/PROTOCOL.md) §9。

## 使用

```bash
# 1. 让当前用户能打开 hidraw（一次性）
sudo cp contrib/udev/70-nuphy.rules /etc/udev/rules.d/
sudo udevadm control --reload && sudo udevadm trigger --subsystem-match=hidraw
# 重新插拔键盘

# 2. 查看
cargo build --release
N=target/release/nuphy
$N info                      # 固件、当前模式、层数、矩阵大小
$N keys                      # 当前模式的底层（--layer N 指定层，--all 全部，--default 出厂值）

# 3. 改键（键名用 QMK 写法：a、esc、lctl、mo(1)、lctl(c)、trns，或十六进制 0x7e06）
$N set --row 3 --col 0 esc            # 当前模式底层：CapsLock → Esc
$N set --layer 0 --row 3 --col 0 esc  # 指定层
$N reset --row 3 --col 0              # 恢复出厂值
$N backup my.bin && $N restore my.bin # 整表备份 / 还原
```

每次写入前都会把整张键位表自动备份到 `~/.local/share/nuphy/backups/`，写完回读确认；改坏了用
`nuphy restore <备份文件>` 还原。`keys` 输出里 `_` 是透明键，`.` 是空位。

## 下一步

GUI（候选：Tauri）→ Windows/macOS 传输（hidapi）→ 宏 / TapDance / 灯光等其余命令。
