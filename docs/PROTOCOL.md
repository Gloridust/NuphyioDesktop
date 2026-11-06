# NuPhy 键盘配置协议（从 NuPhyIO 网页版逆向）

来源：`https://drive.nuphyio.com` 的前端 JS（WebHID，无后端接口）。本文是**静态分析**的结果：
协议细节主要取自站点内置的"虚拟键盘模拟器"（`?isDemoMode=1`）和真实通信层的代码，
**尚未在真机上验证**。已验证 / 未验证的边界见 §9。

## 1. 总览

- 键盘只通过 USB HID 通信，网页用 WebHID 直接读写；没有云端接口参与改键。
- 厂商 ID `0x19F5`。同一个 USB 设备有多个 HID 接口，**只有一个**是配置接口：
  顶层集合 Usage Page `0x01` / Usage `0x00`，无 report id，输入输出报文均 **64 字节**。
- 有两套线协议：**机械款**（Node75、Air75 V3、Halo75 Lite…）和 **Hall 款**（`…HE` 系列）。
  本仓库目前只实现机械款。

## 2. 设备表

`crates/nuphy-proto/src/devices_table.rs` 由站点设备表生成：75 项，含正常模式与 IAP 引导模式（bootloader）PID。
例：Node75 = `0x1030`（引导模式 `0x0730`），Air75 V3 = `0x1028`，Kick75 = `0x1026`，Air75 HE = `0x6120`。

`family`（机械/Hall）是**按型号名推断**的（名字含 `HE` 即 Hall），不是从设备读出的。
`WH80`、`Type2nd` 站点表里没有明确信息，标为 Unknown。

## 3. 传输

- 写：`sendReport(0, 64 字节)`；不足 64 字节补 0。Linux hidraw 下直接 `write(fd, 64 字节)`。
- 读：`inputreport` 事件。首字节 `0xAA` 的是对某条命令的**应答**；其他首字节是**主动上报**（§7）。
- 网页的重试策略：超时 500 ms，最多重试 3 次，间隔 200 ms。写入超时 2 s。

## 4. 帧格式（机械款）

请求（首字节 `0x55`）：

| 偏移 | 含义 |
|---|---|
| 0 | `0x55` |
| 1 | 命令号 |
| 2 | 0 |
| 3 | 校验和 = 第 4..63 字节之和 & 0xFF |
| 4 | 数据长度 ^ key |
| 5,6 | 地址（小端）^ key |
| 7 | handle ^ key |
| 8.. | 数据 ^ key，最多 56 字节，其余补 0（**补 0 部分不异或**） |

应答格式相同，首字节 `0xAA`，第 1 字节回显命令号。

每个命令相当于一块"内存区域"：读 = 指定 `addr` + `len`，一次最多 56 字节，更大的区域需要分块读。
`handle` 的含义因命令而异（模拟器里对数组型区域用作下标，例如按模式分组的灯光/功能配置）；
键位矩阵是否用它表示层，**未确定**。

## 5. 握手（密钥）

连接后先发 `SetSecretKey (0xEE)`：

1. 取 56 个随机字节，填入帧的 8..63；`frame[4]` 为 0；`frame[3]` 照常算校验和。**这一帧不异或。**
2. 密钥 `key = random[20]`（即帧第 28 字节），若为 0 则取 `0xAA`。
3. 等应答后，后续所有帧按 §4 用 `key` 异或。

本仓库在 `Session::open` 里避免 `random[20] == 0`，以免模拟器与真机行为不一致。

## 6. 命令表（机械款）

读类（只读）：`0x2F` GetDongleName、`0xA0` GetBase、`0xA1` GetFirmwareInfo、`0xB1` GetDefaultKeys、
`0xB2` GetUseKeys、`0xB5` GetSOCD、`0xB8` GetTapDance、`0xBB` GetTGL、`0xC2` GetMacro、`0xD1` GetLightCount、
`0xD2` GetKeyLightColor、`0xD5` GetLightState、`0xE1` GetKeyboardFunc、`0xE6` GetTouchBarConfig、
`0xF3` GetSleepInfo、`0xFA` GetAppDefineSize、`0xFB` GetAppDefine、`0xFD` GetDebugEnable。

写类：`0xB3` SetUseKeys、`0xB4` RestoreUseLayers、`0xB6/B7` SetSOCD/Reset、`0xB9/BA` TapDance、`0xBC/BD` TGL、
`0xC3` SetMacro、`0xD6` SetLightState、`0xD8` SetSignalColor、`0xE2` SetKeyboardFunc、`0xE3` SetDebounceTime、
`0xE5` SetTouchBarConfig、`0xF5` SetSleepCfg、`0xFC` SetAppDefine、`0xFE` SetDebugLog。

危险：`0xF1` RestoreFactory（恢复出厂）、`0xEF` SetIapMode（进入固件升级模式）。

### 已知区域布局

- **GetFirmwareInfo**（8 字节，样本 `12 00 01 AA 07 00 00 00`）：版本 = `d[2].d[1].d[0]`，
  当 `d[3] == 0xAA` 时再追加 `.(d[4] | d[5]<<8)`。样本 → `1.0.18.7`。
- **GetBase**（32 字节，样本 `00 08 04 06 10 00 …`）：含义**未解码**。
- **键位矩阵**（GetDefaultKeys / GetUseKeys）：每个键位 2 字节 `[kind, code]`。
  `kind == 0` 时 `code` 是标准 USB HID 用法码（`[0,0x29]` = Esc，`[0,0x35]` = \`）。
  其他 kind（`0x7E`、`0x52`、`0x78`…）是 NuPhy 自定义动作，**未解码**。
  样本长度 1472–1568 字节（736–784 个槽位）；槽位到物理键的映射、层的划分**未确定**。

## 7. 主动上报（首字节 ≠ 0xAA）

`0xA2` 模式切换、`0xC1` 按键上报、`0xD7` 灯光变化、`0xF2` 键盘复位、`0xF4` 睡眠配置变化、`0xFE` 日志。

## 8. Hall（HE）款

命令号见 `cmd::hall`。站点模拟器里的帧：`[0]=0x55/0xAA, [1]=cmd, [2]=key, [3]=校验, [4]=len, [5,6]=addr, [7]=0, [8..]=数据`，
`[2]` 非 0 时数据整体异或该值；校验同样是第 4..63 字节之和。真实客户端的构造代码**没有读**，
所以 Hall 款只有命令表，没有会话实现。

## 9. 验证状态

| 项目 | 状态 |
|---|---|
| WebHID 过滤条件、64 字节无 report id | 来自站点真实通信层代码；Node 75 LP 的 `hidraw6` 描述符吻合 |
| 帧格式、校验和、XOR、握手 | 来自站点真实构造函数 + 模拟器，**单元测试按此编写，未在真机上跑过** |
| 应答校验和 | 只在模拟器里见过，代码里仅报告 `checksum_ok`，不强制 |
| 各命令的数据布局 | 仅 FirmwareInfo / 键位矩阵部分解码，其余未知 |
| 写入类命令 | 本仓库**未实现**，CLI 的 `raw` 只允许读类命令 |

## 10. Linux 权限（Chromium 连不上的原因）

hidraw 节点默认是 `root:root 0600`，且没有 `uaccess` 标签，浏览器和普通进程都打不开。
`contrib/udev/70-nuphy.rules` 给 NuPhy 的 hidraw 加上 `TAG+="uaccess"`：

```bash
sudo cp contrib/udev/70-nuphy.rules /etc/udev/rules.d/
sudo udevadm control --reload
sudo udevadm trigger --subsystem-match=hidraw
```

## 11. 复现分析

1. 抓 `https://drive.nuphyio.com/static/js/main.*.js` 与 `asset-manifest.json` 里的各 chunk。
2. `npx webcrack main.js -o out` 拆 webpack 模块、还原 javascript-obfuscator 的字符串数组。
   个别模块（命令枚举）需把模块单独包一层函数再喂给 webcrack 才能还原。
3. 关键模块（webcrack 输出里的编号）：`97822`（报文/事件枚举）、`99726`（Hall 命令）、`40877`（机械款命令）、
   `36937`（帧构造 / 校验 / 握手）、`65843`（模拟器，等价于协议说明书）、`54391`（设备表）、
   `82037`（HID 传输与重试）。
4. `chunks/291|417|513|820` 是各机型虚拟设备的样本数据（FirmwareInfo、GetBase、键位矩阵）。
