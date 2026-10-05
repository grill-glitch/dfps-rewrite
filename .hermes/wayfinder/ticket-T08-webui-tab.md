# T12 — WebUI: add dfps tab to uperf WebUI

## Question

The endpoint is "WebUI three-tab can switch fps rules". Decide:

1. Add a 4th tab "刷新率" to the existing 3-tab WebUI (首页/模式切换/更多)?
2. The dfps tab reads/writes what files?
3. The dfps-rs daemon needs its own control entry analogous to webui.sh.
   Reuse one shell script or two?

## Constraints

The existing WebUI must continue to work for uperf-only installs. The dfps tab
must work only when the dfps-rs daemon is part of the build.

## Evidence

- webui/index.js:63-77 (existing setupRoute)
- webui/index.html:42-56 (existing 3 pages)
- magisk/script/webui.sh (existing control entry pattern to copy)

## Required output

A short list: tab label, panel content, control entry name.

## Resolution (2026-10-06)

### Tab placement and shape

`webui/route.js` line 7-9 currently registers 3 tabs:

```js
home: { id: 'home-page', title: () => 'Uperf' },
mode: { id: 'mode-page', title: () => getString('tab_mode') },
more: { id: 'more-page', title: () => getString('tab_more') },
```

`webui/index.html` has 3 `<div class="page">` siblings and 3
`.bottom-bar-item` divs. **Add a 4th**, in this order: `home / mode / dfps /
more`. Rationale: `dfps` is a "thing you switch once and forget"; placing
it between `mode` (also a single-axis switcher) and `more` (settings +
log) reads better than putting it last. UI-sensitive — flag for the user
if they don't like it after M4 ships.

### Tab label

`tab_dfps`, i18n keys: `tab_dfps` = "刷新率" (zh) / "Refresh rate" (en). Add
both rows to `webui/language.js`.

### Panel content (single `<div class="page" id="dfps-page">`)

```
section-label: 当前规则 / Current rule
  card: 当前的 <pkg> |  idle Hz   active Hz
        current: <active or idle Hz, read from dfps_cur.txt>
        perapp: list of pkg → (idle, active) from dfps.txt
section-label: 切换 / Switch
  card: list of named-rule buttons (each rule is a card row); click ->
        `setRule <pkg>` writes the new rule file via fwdaction pattern.
section-label: 实时 / Live
  card: live display of dfps_cur.txt's value, auto-refresh every 1s while
        the page is active.
```

A "rule" here means one entry from `dfps.txt`. The tab is read-mostly:
display current state, allow the user to swap which rule a package uses.

### Files the tab calls into (control entry)

Single shell script: `magisk/script/dfps.sh`. Subcommands:

* `dfps.sh status` — prints `cur=<Hz>` read from
  `/sdcard/Android/yc/uperf/dfps_cur.txt`. Used by the live card.
* `dfps.sh info` — prints `lines=<N>; rules=<map>` parsed from
  `/sdcard/Android/yc/uperf/dfps.txt`. Used by the rule list.
* `dfps.sh set-rule <pkg> <idle> <active>` — replaces or appends a line
  in `dfps.txt`. dfps_task's inotify watcher (M2) reloads on close_write
  and the new rule takes effect within ~50ms.
* `dfps.sh restart` — restart daemon (reuses `libuperf.sh:uperf_restart`).

All subcommands follow the existing `key=value` line protocol that
`webui.sh` uses — `webui/ctl.js:36` `await exec(parts.join(' '))` then
parses stdout. dfps tab reuses `ctl.js` by extending its subcommand map
(see patch list).

### `ctl.js` extension (no fork)

```js
const SCRIPTS = {
    uperf: `${MODULE_DIR}/script/webui.sh`,
    dfps:   `${MODULE_DIR}/script/dfps.sh`,
};
export async function runDfpsCmd(sub, ...args) {
    return await runScript('dfps', sub, ...args);
}
```

(`runScript` is the existing generic in ctl.js; if it doesn't exist as a
helper yet, factor it out — that's a 10-line patch.)

### Patch list

```
modify:  webui/route.js                            (add dfps page entry)
modify:  webui/index.html                          (dfps-page div + bottom bar item)
add:     webui/pages/dfps.js                       (render(state, ctl) — same shape as mode.js)
modify:  webui/index.js                            (import dfps.render, call it from setupRoute)
modify:  webui/language.js                         (tab_dfps + dfps page strings)
modify:  webui/ctl.js                              (runScript helper + runDfpsCmd)
add:     magisk/script/dfps.sh                     (status/info/set-rule/restart subcommands)
```

## Outcome

T08 resolved. The "4th tab" requirement in Map destination §3 / AGENT.md §6
is now concretely the `dfps-page`. Files for the tab are listed; their
content lands at M4 (code).
