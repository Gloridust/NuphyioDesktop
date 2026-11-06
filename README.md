# NuphyioDesktop
Nuphyio for Desktop | Linux/Windows/macOS support

NuPhy 官方配置工具 [NuPhyIO](https://drive.nuphyio.com) 是纯前端的 WebHID 应用，Linux 上常因 hidraw 权限连不上。
本项目把它的协议逆向出来，用 Rust 重新实现，目标是做一个原生桌面版。

## 现状

- `crates/nuphy-proto`：协议核心（帧编解码、握手、会话、设备表、键位解析），无 I/O，已有单元测试。
- `crates/nuphy-hid`：Linux hidraw 传输（通过 sysfs 枚举，无需 libudev）。
- `crates/nuphy-core`：CLI 与 GUI 共用的“打开键盘 / 自动备份 / 还原”逻辑。
- `crates/nuphy-gui`：桌面图形界面（Tauri 2 + 原生 HTML/JS，无需 node 构建），界面照 NuPhyIO 网页版的样式做，多语言（目前中文、英文）。
- `crates/nuphy-cli`：命令行工具 `nuphy`：查看（`list` / `info` / `keys` / `get`）、改键（`set` / `reset`）、
  整表备份与还原（`backup` / `restore`）。完全离线，不需要浏览器或网络。
- 只实现了机械款协议（Node75、Air75 V3 等）；Hall（HE）款只有命令表。
- 在 Node 75 LP（固件 1.0.10.6）上验证过读写，其他型号未测，详见 [docs/PROTOCOL.md](docs/PROTOCOL.md) §9。

## 图形界面

```bash
cargo run -p nuphy-gui --release   # 需要 libwebkit2gtk-4.1-dev（Ubuntu：sudo apt install libwebkit2gtk-4.1-dev）
```

和网页版一样：先点上方键盘图中的一个键，再在下方“按键定义”面板里点新的功能，立即写入键盘（写入前自动备份、写完回读确认）。
橙色圆点表示与出厂设置不同；“重置”把选中的键恢复出厂；右上角切换模式（如 M1 Mac / M2 Windows）、语言和查看备份。
旋钮等额外按键在键盘图下方单独列出。在普通浏览器里直接打开 `crates/nuphy-gui/dist/index.html` 会进入演示模式，方便预览界面。

- 键盘外形来自网页版的各型号键位坐标，由 `tools/gen_layouts.py` 生成 `dist/layouts.js`（只保留几何信息和普通按键码）；
  画图前会把坐标和键盘读出的出厂键位逐行对照，对不上就退回按矩阵排列显示。
- 加一种界面语言：在 `dist/locales/` 里照 `en.js` 新建 `<语言代码>.js`，并在 `locales/index.js` 里登记；
  `cargo test -p nuphy-gui` 会检查各语言的键和 `{占位符}` 是否与英文一致。
- 高级按键、灯效、宏、模式设置还没有做（标签页是灰的）。

## 命令行使用

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

Windows/macOS 传输（hidapi）和打包发布→ 宏 / TapDance / 灯光等其余命令。
