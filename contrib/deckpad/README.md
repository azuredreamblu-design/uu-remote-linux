# Steam Deck 原生手柄与震动

[English](#english)

电脑端运行官方 UU 远程，Deck 端运行此 fork 的 SDL 控制端。手柄和震动使用同一个 UU 桌面连接；Windows 不需要额外接收器，出门后无需连接 Deck 的局域网 IP。

## 构建

准备上游 README 所列的 Rust、X11、GTK3 编译依赖，并安装 SDL2 开发库。运行端需要 Python 3、SDL2、GTK3 和支持解码的 ffmpeg；构建依赖的包管理命令随 Linux 发行版不同。以下命令在仓库根目录执行：

```bash
cargo build --locked --release --bin uuremote-server
cargo build --locked --release --features native-viewer,viewer-sdl --bin uuremote-viewer
chmod +x contrib/deckpad/deck_start.sh
```

两个程序应放在同一目录：viewer 的登录流程会复用旁边的 `uuremote-server`。沿用上游登录流程；二维码和登录凭据不要上传到仓库或 issue。

## 从 Steam 启动

1. 把仓库中的 `contrib/deckpad/deck_start.sh` 作为非 Steam 游戏添加到 Steam，目标填写它的绝对路径。
2. 不强制使用 Proton；选择标准游戏手柄的 Steam Input 布局。
3. 打开该入口，在 UU 窗口登录并选择 Windows 主机。
4. 松开所有按键、扳机和摇杆，再开始操作。`Back + Start` 暂停/恢复输入。

默认使用仓库里的 `target/release/uuremote-viewer`。使用其他安装路径时，在 Steam 启动选项中设置：

```text
UU_VIEWER_BIN=/absolute/path/to/uuremote-viewer %command%
```

只关闭震动：`UU_VIEWER_NATIVE_RUMBLE=0 %command%`。请同时打开游戏及 Steam 的震动设置。
启动脚本会管理自己启动的 viewer 进程并记录筛选后的诊断状态；它不会修改 Windows 或启动接收器。
已有旧版 DeckPad 入口的用户请先退出旧入口，只运行一个远程会话；现有安装不会被这个脚本覆盖。

开发时也可以直接运行 `UU_VIEWER_NATIVE_GAMEPAD=1 target/release/uuremote-viewer ui`；Steam 游戏模式建议使用上述脚本以管理退出清理。

## 测试与日志

- Windows `joy.cpl` → 控制器属性：验证按钮、方向键、摇杆及扳机。
- 震动：连接后在 Windows 运行本目录的 `test_rumble.cmd`。等待 8 秒期间让 Deck 的远程画面保持焦点并松开按键。脚本依次测试单路 35%、双路各 45%，最后停止。
- 多个 XInput 控制器时，脚本不会猜测目标；断开其他控制器，或明确使用 `test_rumble.cmd -ControllerIndex 0` 指定正确编号。
- 测试脚本不需要常驻，日常游戏无需启动。用游戏本身的震动动作再验证实际体验。
- 分别测试正常退出、重连及手机热点。休眠/强制退出/突然断网仍需要更多硬件验收。

日志默认在 `~/.local/state/uu-remote-linux/deckpad/session.log`，遵循 `XDG_STATE_HOME`，也可用 `UU_DECKPAD_STATE_DIR` 覆盖。
日志仅允许固定的手柄/震动状态标记及开启诊断后的数值视频标记，不记录按键报告、震动数值或账号令牌。

排查视频时，在 Steam 启动选项设置 `UU_VIEWER_VIDEO_DIAGNOSTICS=1 %command%`。复现后运行 `python3 /absolute/path/to/repository/contrib/deckpad/collect_diagnostics.py`，仅导出最近桌面会话的数值报告。详见[诊断说明](../../docs/steamdeck-video-diagnostics.md)。

`DECKPAD_RUMBLE_RECEIVED` 表示收到有效回传；`DECKPAD_RUMBLE_APPLIED` 表示 SDL 接受了输出。仍以实际震感为准。
`DECKPAD_RUMBLE_OUTPUT_FAILED` 表示本地输出失败，输入继续工作；检查 Steam Input 震动配置后重启入口。

## English

Run the official UU Remote host on Windows and this fork's SDL viewer on the Deck. Input and feedback share the existing desktop connection; no additional Windows receiver or SSH tunnel is required.

Build both sibling binaries from the repository root after installing the upstream Rust/X11/GTK3 dependencies and SDL2 development package:

```bash
cargo build --locked --release --bin uuremote-server
cargo build --locked --release --features native-viewer,viewer-sdl --bin uuremote-viewer
chmod +x contrib/deckpad/deck_start.sh
```

The Deck runtime needs Python 3, SDL2, GTK3 and ffmpeg. Add the absolute path to `contrib/deckpad/deck_start.sh` as a non-Steam game. Do not force Proton; use a standard gamepad Steam Input layout. Connect to Windows, release all controls, then play. Back+Start toggles pause. Only run one remote viewer entry at a time.

The launcher defaults to `target/release/uuremote-viewer`. To override it, set Steam launch options to `UU_VIEWER_BIN=/absolute/path/to/uuremote-viewer %command%`; keep `uuremote-server` beside it for sign-in. Use `UU_VIEWER_NATIVE_RUMBLE=0 %command%` to disable rumble. The launcher supervises its own processes and writes filtered diagnostic markers; it does not install or change Windows components.

Verify input in Windows `joy.cpl`. For a one-time rumble test run `test_rumble.cmd` on Windows while the Deck is connected; refocus the Deck viewer during its eight-second delay. It sends separate 35% motor pulses, then both at 45%, and stops in `finally`. With several XInput devices choose the correct slot explicitly using `-ControllerIndex`. The script is not required for gameplay. Actual feedback depends on hardware and Steam Input settings.

Logs default to `~/.local/state/uu-remote-linux/deckpad/session.log`, respecting `XDG_STATE_HOME` or `UU_DECKPAD_STATE_DIR`. `DECKPAD_RUMBLE_RECEIVED` means valid feedback arrived; `DECKPAD_RUMBLE_APPLIED` means SDL accepted it, not proof of physical vibration. Exit, reconnect, WAN and game-specific feedback should be checked on your own devices. Never upload credentials or raw input reports.

For video diagnostics set Steam launch options to `UU_VIEWER_VIDEO_DIAGNOSTICS=1 %command%`. After reproducing, run `python3 /absolute/path/to/repository/contrib/deckpad/collect_diagnostics.py` to export only the latest desktop session numeric report. The source launcher keeps this feature opt-in.
