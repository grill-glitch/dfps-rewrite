#!/bin/bash
#
# alioth-dfps-smoke.sh — 独立真机冒烟,验证 dfps-rs 的真机行为正确,
# 不依赖 M5 装机(M5 会删 cpp/dfps/、合并进 uperf,这个脚本走的是
# 临时独立路径)。
#
# 三条确认 (2010-10-06 计划 §M3-standalone):
#   1. 装载 /sdcard/Android/yc/uperf/dfps.txt 不报错
#   2. 触发切帧率事件,dfps_cur.txt 被改写
#   3. settings put system peak_refresh_rate 真的改了 dumpsys display modeId
#
# 用法:
#   ADB=~/Android/Sdk/platform-tools/adb \
#   SERIAL=f748d277 \
#   sh scripts/alioth-dfps-smoke.sh
#
# 这个脚本不进 uperf-rewrite 也不进 dfps 模块 magisk 树 — 它只属于
# dfps-rewrite 仓库的 scripts/。

set -eu

ADB="${ADB:-$HOME/Android/Sdk/platform-tools/adb}"
SERIAL="${SERIAL:-}"
ADB_CMD="$ADB ${SERIAL:+-s $SERIAL}"

REMOTE_DFPS_BIN="/data/local/tmp/dfps"
REMOTE_DFPS_TXT="/data/local/tmp/dfps.txt"
REMOTE_DFPS_CUR="/data/local/tmp/dfps_cur.txt"
REMOTE_DFPS_LOG="/data/local/tmp/dfps_log.txt"
USER_PATH="/sdcard/Android/yc/uperf"

# 1. push ELF
echo ">>> Pushing dfps binary to $REMOTE_DFPS_BIN"
$ADB_CMD push "$1" "$REMOTE_DFPS_BIN" 2>&1 | tail -3
$ADB_CMD shell chmod 755 "$REMOTE_DFPS_BIN"

# 2. push minimal dfps.txt (universal + offscreen + 1 rule)
cat <<'EOF' > /tmp/dfps_smoke.txt
/touchSlackMs 4000
/useSfBackdoor 0
* 60 120
- 30 60
com.android.settings 60 120
EOF
$ADB_CMD push /tmp/dfps_smoke.txt "$REMOTE_DFPS_TXT" 2>&1 | tail -1

# 3. kill any prior smoke instance
$ADB_CMD shell "killall -9 dfps 2>/dev/null; true"

# 4. start in background (matches AGENT.md §8: </dev/null >/dev/null 2>&1)
echo ">>> Starting dfps as background process"
$ADB_CMD shell "$REMOTE_DFPS_BIN $REMOTE_DFPS_TXT -o $REMOTE_DFPS_LOG -n $REMOTE_DFPS_CUR </dev/null >/dev/null 2>&1 &"
sleep 2

# 5. check pid (file exists via runnable, not via shell glob)
echo ">>> Pids:"
$ADB_CMD shell "ps -A -o PID,NAME | grep -w dfps || echo no-pid"

# 6. read log
echo ">>> First 30 log lines:"
$ADB_CMD shell "head -30 $REMOTE_DFPS_LOG 2>/dev/null || echo no-log"

# 7. evidence (a): did dfps.txt load?
echo ">>> Evidence (a) — log says 'loaded' or has no error?"
$ADB_CMD shell "grep -E 'loaded|error|fail' $REMOTE_DFPS_LOG | head -5 || true"

# 8. evidence (b): trigger an event (touch) and check dfps_cur.txt
echo ">>> Triggering touch event"
$ADB_CMD shell "input tap 100 100" || true
sleep 1
echo ">>> Evidence (b): dfps_cur.txt content after touch:"
$ADB_CMD shell "cat $REMOTE_DFPS_CUR 2>/dev/null || echo no-cur-file"

# 9. evidence (c): dumpsys display — see peak_refresh_rate effect
echo ">>> Evidence (c): dumpsys display | grep -E 'modeId|refresh'"
$ADB_CMD shell "dumpsys display | grep -E 'modeId|peak_refresh_rate|refresh' | head -10" || true

# 10. cleanup
echo ">>> Cleaning up"
$ADB_CMD shell "killall -9 dfps 2>/dev/null; rm -f $REMOTE_DFPS_BIN $REMOTE_DFPS_TXT $REMOTE_DFPS_CUR $REMOTE_DFPS_LOG"

echo ">>> Done. Check the output above for the 3 evidence points."