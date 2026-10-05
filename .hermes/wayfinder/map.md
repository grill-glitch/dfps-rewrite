# Wayfinder map — dfps-rs 重写

## Destination

把 dfps 完整用 Rust 重写、嵌入现有 uperf KernelSU 模块。验收：
1. **alioth 上切帧率路径可用 [V]** — sf 后门 (`service call SurfaceFlinger 1035`)
   实测拒绝（root + Enforcing），终点的切帧率路径退回到 **`settings put system
   peak_refresh_rate`（小米路径）** 或 vendor-specific 设置；接受此 fallback 后
   的兜底验收。
2. AGENT.md 有 dfps-rs 里程碑表（M-列 + V/I/U 状态标注）
3. WebUI 增加 "刷新率" 标签页，可从 KernelSU 管理器里切换 fps 规则

## Notes

- 工作目录 `~/uperf-rewrite`，fork `grill-glitch/uperf-rewrite`，分支 `game-turbo`
- 所有 Rust crate 命名沿用 `uperf-*` 前缀（`uperf-config`/`uperf-core`/`uperf-cli`）
- 装机模式：embedded（共用 `magisk/` 模块树、`USER_PATH=/sdcard/Android/yc/uperf`）
- 真机验证目标：alioth (`f748d277`)
- 引用资料：原 `cpp/dfps/` 即将被删除，本次重写后**仓库里不应该再出现 `cpp/dfps/`**

## Decisions so far

<!-- 一行/ticket：gist + 链接 -->

- **[T01: sf-backdoor-probe]** — 结论：**sf 后门在 alioth 不可用**（root + SELinux Enforcing
  下 `service call SurfaceFlinger 1035` 5/5 全部 `Operation not permitted`，未观察到
  `dumpsys display` 的 modeId 变化）。dfps-rs 的默认切帧率路径**必须 fallback 到
  `settings put system peak_refresh_rate`**（小米）或 vendor-specific 设置。T01 子代
  理仍在写最终 report（取 AVC detail），结论不变。
- **[T02: config-format]** — 结论：上游 dfps 是个**独立模块**，
  USER_PATH `/sdcard/Android/yc/dfps/{dfps.txt, dfps_log.txt, dfps_cur.txt}`。
  配置是 256 字节/行的文本：注释 `#`，tunable 前缀 `/`，规则 `<pkg> <idle> <active>`。
  特殊包名 `*`（万能规则）和 `-`（熄屏规则）。
  详：`/tmp/dfps-config-format.md`。
- **[T03: ipc-or-same-binary]** — 结论：**(c) dfps-rs 跑在同一 Rust 二进制里**。
  理由：(a) `sfanalysis.hint` 这条 notify-file 协议已经承载多字节但语义与帧率不同，
  复用会冲突；(b) Unix socket 对一个 userspace 模块是过度工程；(c) 现有
  `topic_dispatch.rs` 已能将 cgroup/input/offscreen 派发给多订阅者，dfps 只是再加
  一个 orchestrator 订阅者，订阅 4 个已有 topic（input.touch/input.btn/
  topapp.*/offscreen.state）。同进程也消除双进程 supervisor 的复杂度。
- **[T04: data-structures]** — 结论：`HashMap<String, FpsRule>` + 两个
  `Option<FpsRule>` 字段（`universial`/`offscreen`）；`FpsRule { idle: i32, active: i32 }`
  逐字；`curHz_` 哨兵值换成 `Option<i32>`；`forceSwitch_` 用 `Cell<bool>`（heavy-worker
  单线程）。详见 ticket-T04-data-structures.md。
- **[T05: switch-call-frequency]** — 结论：保留上游 `(hz != curHz_) || force` 去重，
  保留 `force=true` 参数（3 个真实调用者：TopApp 切换 / 熄屏进入 / 熄屏唤醒）；
  `forceSwitch_` 模块字段折进闭包捕获；settings-put 路径不额外加节流。
  详见 ticket-T05-switch-call-frequency.md。
- **[T06: notify-file-path]** — 结论：`/sdcard/Android/yc/uperf/dfps_cur.txt`
  （与 uperf-rs 同一 `USER_PATH`，不复用旧 `/sdcard/Android/yc/dfps/`）。
  详见 ticket-T06-notify-file.md。
- **[T07: module-merge]** — 结论：单二进制 `bin/uperf`（不增 `bin/dfps`）；
  service.sh 不变；module.prop bump versionCode + "+ dfps (Rust)"；
  customize.sh 增加 `dfps.txt` 缺失时播种；NOTICE 删除 spdlog/scnlib/dfps 三项。
  详见 ticket-T07-module-merge.md。
- **[T08: webui-tab]** — 结论：第 4 个 tab 插在 `mode` 与 `more` 之间，
  i18n key `tab_dfps`；面板含当前规则 / 切换 / 实时三段；
  新增 `magisk/script/dfps.sh`（status/info/set-rule/restart 子命令，`key=value`
  协议与 webui.sh 一致）；`ctl.js` 扩 `runScript` + `runDfpsCmd`。
  详见 ticket-T08-webui-tab.md。
- **[T09: build-wiring]** — 结论：dfps-rs 放进 `uperf-core` 新模块
  (`dfps_task.rs` + `dfps_config.rs`)，不新开 crate；CMakeLists 删 dfps 子目录
  + 删 cpp/uperf/app_main.cpp 的 `-o/-n` argv；build.sh 的 cargo test
  加两行（dfps_config/dfps_task）；NOTICE 删 3 项。
  详见 ticket-T09-build-wiring.md。

## Tickets

### 已决（依据研究结论）

- **[T01]** ✓ — sf 后门不可用（已决，影响 T06 默认路径）
- **[T02]** ✓ — 配置格式与 USER_PATH（已决，影响 T06/T07/T11）
- **[T03]** ✓ — 同二进制（已决，影响 T07）
- **[T04]** ✓ — 数据结构（已决，影响 T05/T06/T07 — 已解锁）
- **[T05]** ✓ — SwitchRefreshRate 频控（已决，影响 dfps_task.rs 实现）
- **[T06]** ✓ — notify 文件路径（已决，影响 dfps_task.rs 实现）
- **[T07]** ✓ — module 合并策略（已决，影响 cpp/ + magisk/ 全套）
- **[T08]** ✓ — WebUI 第 4 tab（已决，影响 webui/ + magisk/script/dfps.sh）
- **[T09]** ✓ — build 集成（已决，影响 build.sh + CMakeLists）

### 未决

（无。前沿已清空到终点）

## Not yet specified

（fog 在 `.wayfinder-fog.md`，现已全部化为 ticket + 决议，没有未指定的雾）

## Out of scope

- 制作独立的 dfps 模块 zip（"embedded" 排除此路）
- 重写 uperf 的 C++ 主进程（dfps 重写 ≠ uperf 重写）
- 在 dfps 里实现 uperf 才有的功能（CPU 调度、调度器、atrace、log.level 等）
- 修改 dfps 的上游历史（重写不保留任何 vendored cpp/dfps）
- 在其它设备上验证（仅 alioth）
