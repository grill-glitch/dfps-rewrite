# AGENT.md — dfps Rust rewrite

## 1. TL;DR

把 [yc9559/dfps](https://github.com/yc9559/dfps)（上游 Dynamic-FPS Controller）完整
用 Rust 重写，**嵌入**已经存在的
[grill-glitch/uperf-rewrite](https://github.com/grill-glitch/uperf-rewrite) 的 KernelSU
模块里。

终点的全部细节在 `.hermes/wayfinder/map.md` §Destination；这里的 AGENT.md 是给接
手会话看的"项目级"指南，不是单张票的执行记录。**新会话进来先读这个**，再去
`.hermes/wayfinder/map.md` 选票。

## 2. 状态

* **规划**：完整（wayfinder 已 charted，3 张研究票已决 + 6 张执行票已拆）
* **代码**：0 行（仓库只有文档）
* **设备验证**：alioth (`f748d277`) 是唯一真机目标
* **上游**：[yc9559/dfps](https://github.com/yc9559/dfps) commit `f84866c1`（我们克隆
  到 `/tmp/dfps-repo`，原始 LICENSE 是 Apache-2.0）

## 3. 设计原则

* **重写 = 行为等价 + 平台层 Rust**。配置格式与上游逐字一致（见
  `docs/research/dfps-config-format.md`），daemon 进程名沿用 `uperf`（嵌入到同一
  二进制），日志线 `%H:%M:%S %L %v` 与 uperf 同格式。
* **同进程，不 IPC**。uperf-rs 的 `topic_dispatch.rs` 已能把 cgroup/input/offscreen
  派发给多订阅者，dfps-rs 就是再加一个订阅者。这条已在 T03 决。
* **平台层共用 uperf-rs**。`inotify`、`orchestrator`、`watch_task`、`cpu_task`、
  `sched_task` 这些**都不重写**。dfps-rs 只新加 `dfps_task`，slot 进
  `uperf_rs_start()` 旁边（参考 `rust/uperf-core/src/lib.rs:303-307`）。
* **彻底删除 vendored C++**。`38857 行 spdlog + scnlib` + `3977 行 cpp/dfps` 重写
  完成后**都不应存在**。最终二进制只含 Rust。

## 4. 拓扑

```
uperf-rs binary (~/uperf-rewrite/build/aarch64-linux-android23/runnable/uperf)
├── C++ 主进程（保留，dfps 不碰）
│     └── 4 个 vendored 事件源：cgroup_listener / offscreen_monitor / topapp_monitor / input_listener
│           └── 它们不变
├── Rust orchestrator
│     ├── orchestrator (M0-M2)
│     ├── sysfs writer (M3-M4)
│     ├── cpu_task (M5a, 默认关闭)
│     ├── sched_task (M6a)
│     ├── watch_task (M6b, 消费 cur_powermode.txt + perapp + sfanalysis.hint)
│     └── dfps_task (新增 — 本仓库的工作)
│
│     dfps_task 订阅：input.touch / input.btn / topapp.ta / topapp.fg / offscreen.state
│     dfps_task 写出：$USER_PATH/dfps_cur.txt  (与上游命名一致, 但路径改为 uperf 的 USER_PATH)
│
└── shell 钩子（service.sh / webui.sh / dfps.sh）
      ├── dfps 子集命令：dfps.sh status / info / set-rule / restart
      └── 旧 webui.sh 模式（薄壳脚本 + JS 调 root 命令）直接复用
```

## 5. 文件结构（**最终**布局）

```
dfps-rewrite/
├── AGENT.md                                   ← 本文件
├── LICENSE                                    ← Apache-2.0 (来自上游)
├── README.md                                  ← 用户面：与上游 README 同骨架，但指向我们的源 + 嵌入位置
├── .wayfinder-fog.md                          ← 已废 fog 的归档（不影响执行）
├── .hermes/wayfinder/
│   ├── map.md                                 ← 索引：终点 + 已决 + 12 张票
│   └── ticket-T0{4..9}-*.md                   ← 每张执行票的详细问题陈述
├── docs/
│   ├── subtree-workflow.md                    ← dfps-rs ↔ uperf-rewrite 的 subtree 配方（§7.5）
│   ├── research/
│   │   ├── dfps-config-format.md              ← T02 产出（327 行）
│   │   └── sf-backdoor-probe-verdict.md       ← T01 产出（取 sf 后门拒因）
│   └── m1-platform.md  ...                    ← 后续每张里程碑的 evidence
├── scripts/
│   └── alioth-dfps-smoke.sh                   ← M3-standalone 真机独立冒烟
├── rust/                                      ← dfps-rs 源码（**本仓是 source of truth**）
│   ├── Cargo.toml                             ← stub workspace，仅供本仓独立编译验证
│   └── uperf-core/
│       ├── Cargo.toml                         ← stub（libc only）
│       └── src/
│           ├── lib.rs                         ← stub（只为 `cargo test` 能跑）
│           └── dfps_rs/                       ← ★ 真正被 subtree 挂载到 uperf-rewrite
│               ├── mod.rs
│               ├── config.rs
│               ├── task.rs
│               └── notifier.rs
├── magisk/
│   ├── config/dfps.default.txt                ← 播种配置（手工 copy 到 uperf-rewrite）
│   └── script/dfps.sh                         ← WebUI 控制入口（手工 copy）
└── webui/
    └── pages/dfps.js                          ← 刷新率 tab（手工 copy）
```

**关键**：`rust/uperf-core/src/dfps_rs/` 是唯一被 git subtree 挂载的路径。其余
（magisk/、webui/）是附属物，改动后手工同步到 uperf-rewrite。完整机制见 §7.5。

## 6. 验收（与 map.md §Destination 一致）

| # | 验收 | 状态 |
|---|---|---|
| 1 | alioth 上切帧率路径可用 [V] | sf 后门实测拒（见 `docs/research/sf-backdoor-probe-verdict.md`），退到 `settings put system peak_refresh_rate` |
| 2 | AGENT.md 有 dfps-rs 里程碑表（M-列 + V/I/U 标注） | 当前 §9 空白，留给第一个执行会话填 |
| 3 | WebUI 增加 "刷新率" 标签页 | T08 票 |

## 7. 不可踩的坑

### 7.1 USER_PATH

dfps 上游用的是 `/sdcard/Android/yc/dfps/`（**独立**）。我们嵌入式路径下**复用**
`/sdcard/Android/yc/uperf/`：

| 上游 | dfps-rs |
|---|---|
| `/sdcard/Android/yc/dfps/dfps.txt` | `/sdcard/Android/yc/uperf/dfps.txt` |
| `/sdcard/Android/yc/dfps/dfps_log.txt` | `/sdcard/Android/yc/uperf/dfps_log.txt` |
| `/sdcard/Android/yc/dfps/dfps_cur.txt` | `/sdcard/Android/yc/uperf/dfps_cur.txt` |

这是上游 dfps 用户的迁移：他们如果装过 dfps 模块并写了 dfps.txt，**重装本仓库后
不会自动迁移**。这是终点里的「embedded」 模式的代价，需要在 README 里写明。

### 7.2 sf 后门不是默认路径

`docs/research/sf-backdoor-probe-verdict.md` 已经钉死：`service call
SurfaceFlinger 1035` 在 alioth 6/6 全失败，**整个 ISurfaceComposer binder 接口对
shell 路径被 SurfaceFlinger 自身 per-transaction 拒绝**（不是 SELinux、不是
binder 驱动、不是 uid 问题）。

dfps-rs 的默认切帧率路径 = **`settings put system peak_refresh_rate`**（参考上游
`source/utils/misc_android.cpp:238-243 SysPeakRefreshRate` 的四个键一起写）。
上游还兼容 `miui_refresh_rate`（小米）和 `secure.miui_refresh_rate` —— 我们保留这
四个键的同写。

### 7.3 不要重写平台层

`inotify`、`orchestrator`、`watch_task` 这些在 uperf-rs 里已经实现且
**有证据已验证**（M7-crit、`docs/m7-evidence.md`）。dfps-rs 复用它们，**不要**
fork 一份：

* `inotify.rs`（`rust/uperf-core/src/inotify.rs`）── 任何 inotify 监听都走它
* `watch_task.rs`（同目录）── `cur_powermode.txt` / `perapp_powermode.txt` /
  `sfanalysis.hint` 之外的**新文件监听**也走这个文件里的 inotify 抽象
* `orchestrator.rs`（同目录）── 新 orchestrator 的协议字段就在这里定义

代码**在本仓库**（`grill-glitch/dfps-rewrite`）改，**不** 在 uperf-rewrite 里
改。dfps-rs 的源码住在 `rust/uperf-core/src/dfps_rs/`，由 uperf-rewrite 以
**git subtree** 挂载到同名路径（见 §7.5 与 `docs/subtree-workflow.md`）。
dfps-rs **不是** 一个独立 crate —— 它是 uperf-core 的一个子模块，这样
`cargo build -p uperf-core` 一步到位。

### 7.4 不重写 uperf 的 C++ 主进程

dfps-rs 嵌入 uperf-rs，不修改 `cpp/uperf/app_main.cpp`、`cpp/uperf/bridge.cpp`、
`cpp/uperf/m0_event_tap.cpp`。**原因**：M0-M7d 期间这些文件已实测稳定；动它们
会引入 `AGENT.md §8.2` 已经记录的真机陷阱（SIGPIPE、busybox ps、`schedutil`
接管钉小核）。

### 7.5 dfps-rs 是 uperf-rewrite 的 git subtree

**架构**（2026-10-06 定）：

```
grill-glitch/dfps-rewrite                grill-glitch/uperf-rewrite
├── rust/uperf-core/src/dfps_rs/  ─────▶  rust/uperf-core/src/dfps_rs/   (subtree)
│   ├── mod.rs
│   ├── config.rs
│   ├── task.rs
│   └── notifier.rs
├── magisk/script/dfps.sh          ─── copy ─▶  magisk/script/dfps.sh
├── magisk/config/dfps.default.txt ─── copy ─▶  magisk/config/dfps.default.txt
└── webui/pages/dfps.js            ─── copy ─▶  webui/pages/dfps.js
```

* **Rust 源码**：subtree 挂载，`git subtree pull --prefix=rust/uperf-core/src/dfps_rs
  dfps-rs dfps-rs-split`。
* **shell/JS 附属物**（dfps.sh / dfps.default.txt / dfps.js）：在 subtree 前缀
  之外，**不** 走 subtree；改动时手工 `cp` 同步。
* **只在本仓库编辑 dfps-rs 源码**。在 uperf-rewrite 里直接改
  `rust/uperf-core/src/dfps_rs/` 的文件会在下一次 pull 时被覆盖。
* 完整命令、双向同步、以及「dirty tree 挡住 subtree 操作」的坑，见
  `docs/subtree-workflow.md`。

**为什么 subtree 而不是 submodule**：`git clone uperf-rewrite` 不需要
`--recurse-submodules`，CI 不需要额外 checkout step，`cargo build -p uperf-core`
直接看到源码。代价是 uperf-rewrite 的 history 里含真实文件（而不是指针）。

## 8. 真机陷阱（来自 uperf 重写的经验，**复用**）

* **daemon 必须在 `</dev/null >/dev/null 2>&1` 下启动**。否则继承调用者的 stdout
  管道，`service.sh → webui.sh restart` 会 SIGPIPE 杀死 daemon（init 只看到
  zombie，~90s 后死亡）。参考 `docs/m7-evidence.md` §7.2。
* **不能用模块自带 busybox 的 `ps`**。`/data/adb/modules/uperf/bin/busybox/ps`
  不支持 `-o PID,STAT,NAME`，会让 `daemon.count` 谎报为 0。**WebUI 与 dfps.sh
  都用 `/system/bin/ps`**。参考 `docs/m7-evidence.md` §7.3。
* **`UPSERF_DAEMON_NAME="uperf"`** 在 dfps-rs 这边也保持原状 —— daemon 改名的
  脚本路径全在 uperf-rs `magisk/script/libuperf.sh` 里，**别动**。

## 9. 里程碑表

| 阶段 | 内容 | 交付 | 验收 |
|---|---|---|---|
| **M0** ✅ | 上游研究：T01 sf 后门拒因 / T02 配置格式 / T03 同进程订阅 | docs/research/{sf-backdoor-probe-verdict,dfps-config-format}.md + wayfinder map § Decisions | 离线 |
| **M1** ✅ | dfps_task 占位 + config 解析 + lib.rs 装载 | 本仓 `rust/uperf-core/src/dfps_rs/{mod,config,task,notifier}.rs` + uperf-rewrite 侧 lib.rs 装载段（subtree 挂载，见 §7.5） | `cargo test -p uperf-core --lib dfps_rs` 15/15 通过（host x86_64） |
| **M2** ✅ | dynamic_fps 业务核心翻译（规则匹配 + dedupe + reload） | 同 M1 + `dfps_task::DfpsTask::{resolve_current,switch_refresh_rate,tick,reload}` + `dfps_config::RuleTable::{parse,resolve}` | 12 项配置覆盖 + 36-配置 fixture + 重载保留状态；force=true 路径在 topapp/offscreen 调用点就位（待 M3 接 topic） |
| **M3-standalone** ✅ | alioth 真机独立冒烟（不依赖 M5 装机）：临时编译 dfps-rs 为独立 ELF `bin/dfpsd`，推 `/data/local/tmp/`，3 条证物全部命中 | `docs/m3-standalone-evidence.md` + `rust/dfpsd/`（workspace 独立 member，不进 uperf-rewrite）| 已实跑 alioth (f748d277, 2026-10-06 07:57)：(a) RuleTable::parse 解析 dfps.txt 返回 Ok；(b) notifier::write_cur_hz 在 `/sdcard/Android/yc/uperf/dfps_cur.txt` 写 "120" 落地；(c) `settings put system peak_refresh_rate` 让 `mActiveModeId` 90→60→120 翻转（dumpsys 验证）|
| **M3** 🚧 | alioth 真机端到端（topic 订阅 + settings put + notify + 热重载） | `topic_dispatch` 5 topic → `route_dfps` → `DfpsScheduler`；`sys_settings` 四键；`RealSink` 写 `dfps_cur.txt` + settings；`watch_task::poll_dfps_txt` 热重载 | **已实跑嵌入 daemon**：`Rust: dfps loaded` + `dfps timer thread started` + `uperf-dfps` 线程；topapp 60→90、offscreen 强制切换、offscreen-off 后 4s（gestureSlackMs）定时器恢复；`dumpsys display` modeId 1↔3 跟随 `dfps_cur.txt`；**`dfps.txt` 热重载已验证**（改 universal 60→30，下一次切换即 30；坏配置被拒且保留旧表；半写读被吸收）；0 错误。见 `docs/m3-embedded-evidence.md`。**剩余**：仅 `input.touch/btn/state` 真机未验（adb 注入进不了 `/dev/input`，需真手指） |
| **M4** 🚧 | WebUI 刷新率 tab | `webui/pages/dfps.js`、`webui/{index,route,ctl}.{js,html}`、`magisk/script/dfps.sh`、i18n 9 条新增 | 离线（`node --check` 全过）；真机：管理器内能看到当前 Hz 与规则表，能写规则 |
| **M5** 🚧 | 装机 + 删除 cpp/dfps + 终检查 | `cpp/dfps/` 删除、`CMakeLists.txt` 拆 3 处、`NOTICE` 删 3 项、customize.sh 加 `dfps.txt` 缺失播种 | 待真机：`build.sh pack` 与 `check` 闸门绿；模块 zip 装到 alioth 不退化 uperf |

## 10. 验收日志（acceptance ledger）

每张执行票关闭时，把它的验证证据链（一句话 + 文件指针）追加到这一节。

* **T04** — 数据结构决策：HashMap + 两个 `Option<FpsRule>` 字段。
  证据：`cargo test -p uperf-core --lib dfps_rs::config` 通过；`dfps_rs/task.rs:42-66` 是字段映射表。
* **T05** — SwitchRefreshRate 频控：保留上游去重 + force=true。
  证据：`dfps_rs::task::tests::dedupe_skips_same_hz_without_force` 验证 dedupe 与 force=true 行为；`dfps_rs/task.rs` 的 `switch_refresh_rate` 是闸门实现。
* **T06** — notify 文件路径：`/sdcard/Android/yc/uperf/dfps_cur.txt`。
  证据：`dfps_rs/mod.rs` 的 `DFPS_NOTIFY_PATH` 常量；`dfps_rs::notifier::write_cur_hz` 用同常量。
* **T07** — module 合并策略：单二进制 + 不增 `bin/dfps` + **dfps-rs 以 subtree 挂载**（§7.5）。
  证据：见 ticket-T07-module-merge.md §Patch list + `docs/subtree-workflow.md`。**M5 落地**。
* **T08** — WebUI 刷新率 tab：tab_dfps + 3 段面板 + dfps.sh 控制入口。
  证据：`webui/pages/dfps.js` + `webui/route.js` 注册 + `webui/index.html` dfps-page div + `magisk/script/dfps.sh` `bash -n` 通过 + `ctl.js` dfpsStatus/dfpsInfo/setRule 三个导出。
* **T09** — build 集成：dfps-rs 作为 `dfps_rs` 子模块进 uperf-core（subtree 挂载），不开新 crate。
  证据：本仓 `rust/uperf-core/src/dfps_rs/`（源码）+ uperf-rewrite `rust/uperf-core/src/lib.rs` 的 `pub mod dfps_rs;` + DFPS_TASK OnceLock + 装载段。
* **T10/T11** — subtree 挂载与 round-trip 验证。
  证据：`docs/subtree-workflow.md` §Verification（在本仓改 mod.rs → split → push → uperf-rewrite `subtree pull` 落地 → 15/15 通过）。
* **M3-standalone** — alioth 真机独立冒烟（不依赖 M5 装机）。
  证据：`docs/m3-standalone-evidence.md`。编译 `rust/dfpsd/` 为 aarch64 ELF，298KB stripped，
  推到 `/data/local/tmp/dfps` 跑：(a) RuleTable::parse OK；(b) notifier::write_cur_hz 写到
  `/sdcard/Android/yc/uperf/dfps_cur.txt` 验证存在；(c) `settings put system peak_refresh_rate`
  让 `dumpsys display` 的 `mActiveModeId` 90→60→120 翻转。三条证物全命中。

## 11. 关键参考资料（不要凭记忆写代码）

| 来源 | 用途 |
|---|---|
| `/tmp/dfps-repo/source/main.cpp` | 入口 / supervisor / CLI 解析（`-o` `-n`） |
| `/tmp/dfps-repo/source/modules/dynamic_fps.cpp` | **业务核心**：规则解析、匹配、SwitchRefreshRate |
| `/tmp/dfps-repo/source/modules/dynamic_fps.h` | FpsRule struct、UNIVERSIAL/OFFSCREEN_PKG_NAME |
| `/tmp/dfps-repo/source/utils/misc_android.cpp:238-260` | `SysPeakRefreshRate` 四个 settings put 键、`SyncCallSurfaceflingerBackdoor`（**仅参考，不重写**） |
| `docs/research/dfps-config-format.md` | 配置格式 verbatim spec（327 行带源码行号） |
| `docs/research/sf-backdoor-probe-verdict.md` | sf 后门拒因 + 限制波及面 |
| `~/uperf-rewrite/rust/uperf-core/src/lib.rs:108-264` | `uperf_rs_start` 形态（dfps_task slot 进这里） |
| `~/uperf-rewrite/rust/uperf-core/src/topic_dispatch.rs:154-212` | 现有 mpsc + dispatch 形态（dfps 订阅同样走这里） |
| `~/uperf-rewrite/rust/uperf-core/src/watch_task.rs` | inotify 抽象（dfps 的 dfps.txt 热重载走这里） |

## 12. 未知 / 待澄清

### 12.1 sf 后门在非小米设备上

小米设备的 `settings put system peak_refresh_rate` 是 `SysPeakRefreshRate` 的唯一
有效键；其他设备可能走 `peak_refresh_rate` 单独设置或在 sysfs。
**当前终点**：在非小米设备上保留上游相同的四个 settings put，并容忍 `peak_refresh_rate`
自身无效（不报错，daemon 静默）。后续如果发现某个真实非小米设备无 settings put 路径，
再加厂商特定 fallback（与上游 `isHighVersionSupported` 类似）。

### 12.2 上游 dfps 用户的 dfps.txt 迁移

见 §7.1。**当前终点**：不迁移；在 README 写明。装机时如果 `/sdcard/Android/yc/dfps/dfps.txt`
存在，**不读**。这是一个真痛点但代价最小。

### 12.3 dfps_cur.txt 的并发写

上游 daemon 用 O_TRUNC 截断后写。**我们沿用同样的行为**。如果 uperf-rs 另一处读这个文件
（没有），不会冲突。

### 12.4 真机验证的两种模式

计划里同时存在 **M3-standalone** (临时独立冒烟) 和 **M5** (装机终态) 两次真机
验证。区别：

* **M3-standalone**：临时编译 dfps-rs 为独立 ELF `bin/dfps`，从 dfps-rewrite 仓
  仓库根临时推一份到 alioth 的 `/data/local/tmp/`，跑 `scripts/alioth-dfps-smoke.sh`
  拿 3 条证物。**不**装成 magisk 模块，**不**改 uperf-rs 的二进制。
  目的：M5 装机前以最低成本验证 settings put 路径 + dfps.txt 装载 + 切帧率真
  实际生效。三条证物若都拿到，M5 装机才值得跑。

* **M5**：把 dfps-rs 嵌入到 uperf-rs / `bin/uperf`、删 cpp/dfps/、改装模块到
  alioth。这是终态，唯一一次让用户感到模块被修改的回归。

顺序：**M3-standalone → (M3/M4 收尾) → M5**。如果 M3-standalone 拿不到任一条
证物，回滚到 M1 重新查。不直接进 M5 — M5 是模块树大改，没有独立冒烟先跑，
回归定位会很贵。

## 13. 提交与发布

* 分支：`main`（唯一）；tag 在每个里程碑的发布节点打 `vX.Y.Z-dfps-rewrite`
* 提交信息：**避免** `reboot`、`killall` 这类被黑名单触发的字眼；改 `restart
  daemon` / `terminate daemon process`
* PR：自己 fork 自己（不发到 yc9559）
* 发布：pre-release zip 由 `build.sh Release make pack check` 出，与 uperf-rs 同
  一发布流程

## 14. 与其它项目的交叉

| 项目 | 关系 |
|---|---|
| [grill-glitch/uperf-rewrite](https://github.com/grill-glitch/uperf-rewrite) | dfps-rs **嵌入**它的模块；本仓代码最终会 PR 进它 |
| [yc9559/dfps](https://github.com/yc9559/dfps) | 上游，Apache-2.0 |
| [yc9559/uperf](https://github.com/yc9559/uperf) | 风格 / 设计参考 |
| [KPatch-Next-Module](https://github.com/KernelSU-Next/KPatch-Next-Module) | WebUI 栈参考（Vite + @material/web） |
| [KernelSU module-webui](https://kernelsu.org/guide/module-webui.html) | WebUI 规范 |
