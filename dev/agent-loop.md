# FigCraft 实时联调 · 任意 Agent 通用指南

> 任何能跑 shell 命令的 Agent（Cursor、Claude Code、ZCode、Cline、Codex、终端）都按本文件操作。
> 原则：**只依赖 shell + 固定端口**，不依赖任何特定 IDE；支持 MCP 的 Agent 可选用 MCP。

## 常驻服务（一次性安装，之后"随时"可用）

```sh
Figcraft/dev/figctl install     # LaunchAgent 常驻：FigCraft --control 7979，崩溃/重启自动拉起
Figcraft/dev/figctl position right   # 窗口钉在屏幕右半边（IDE 占左边）
```

- 之后**任意时刻**任意 Agent 都通过 `127.0.0.1:7979` 与真实窗口联调；用户在右侧窗口里实时看到 Agent 的每个操作。
- 卸载：`figctl uninstall`；日志：`~/Library/Logs/figcraft-dev.log`（`figctl log`）。

## Agent 联调循环（功能开发）

```sh
# 1. 改 Rust 代码（IDE / Agent 直接编辑）
# 2. 编译并重启常驻窗口（右侧窗口原地刷新，自动归位右半屏）
Figcraft/dev/figctl build
Figcraft/dev/figctl restart

# 3. 重建现场（引擎命令即刻生效，用户实时可见）
Figcraft/dev/figctl run file.new
Figcraft/dev/figctl run text.create '{"text":"Hello quik world","x":60,"y":60}'

# 4. 看像素（截图回给 Agent 自己"看"）
Figcraft/dev/figctl shot /tmp/fc.png

# 5. 交互复现（键鼠级联调）
Figcraft/dev/figctl key Enter
Figcraft/dev/figctl type " quik"
Figcraft/dev/figctl rclick 400 550      # 右键开上下文菜单
Figcraft/dev/figctl click 312 46        # 点菜单行

# 6. 读状态（数值级核对）
Figcraft/dev/figctl inspect             # 全量 UI/文档状态 JSON（含 t3 弹窗开合与 rect）
Figcraft/dev/figctl run node.get '{"id":2}'

# 7. 直接打开浮窗（免去像素找控件；选中 3D transform 效果后）
Figcraft/dev/figctl exec ui.set '{"t3Popover":{"id":2,"index":0}}'   # null 关闭
Figcraft/dev/figctl exec ui.set '{"t3Settings":true}'                # 主浮窗需已打开
Figcraft/dev/figctl exec ui.set '{"t3Pivot":true}'                   # 九点转动点浮窗（独立面板）
```

## 坐标换算（重要）

截图是物理像素，点击是 egui 逻辑点：**点击坐标 = 截图像素 ÷ `figctl scale` 的 `drawn` 值**（Retina 通常 2.0）。
`figctl scale` 直接打印该值。

## 更快的一档：无头 UI 测试（不弹窗、秒级）

UI 行为回归不必开窗口：仿照 `crates/ui-egui/src/spell_menu_tests.rs` 写 `*_tests.rs`
（egui headless 断言，仓库既有模式），然后：

```sh
cargo test -p figcraft-ui-egui --lib spell_menu   # Agent 改完代码先跑这个
```

约定节奏：**改代码 → cargo test（逻辑秒验）→ figctl build + restart → figctl shot（像素终验）**。

## MCP（可选，支持 MCP 的 Agent 体验更好）

```bash
# Claude Code
claude mcp add figcraft-app -- <repo>/target/debug/figcraft-cli mcp --attach 7979
```

Cursor：`.cursor/mcp.json`；VS Code：`.vscode/mcp.json`：

```json
{ "mcpServers": { "figcraft-app": {
    "command": "<repo绝对路径>/target/debug/figcraft-cli",
    "args": ["mcp", "--attach", "7979"] } } }
```

MCP 工具：`run_command` / `get_node` / `inspect_document` / `screenshot {window:true}` /
`pointer` / `key` / `export`（全表见 `docs/mcp.md`）。attach 模式下 App 会在人员栏显示
Agent 头像，并高亮它改动的图层 —— 用户看得见 AI 在做什么。

## 提交纪律

功能开发在 `dev/<功能名>` 分支进行，一个功能点一个提交，随做随推（实时代码保存）；
不动 `main`。提交信息用英文祈使句，风格对齐上游。
