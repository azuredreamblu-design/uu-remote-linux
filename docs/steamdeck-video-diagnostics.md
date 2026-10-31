# Steam Deck 视频诊断 / Video diagnostics

This diagnostic branch adds opt-in counters to the native viewer. Set
`UU_VIEWER_VIDEO_DIAGNOSTICS=1` before launching the viewer. The packaged Steam Deck
launcher sets this automatically and enables the network panel for that run.
This is an instrumented build, not a demonstrated frame-rate fix.

设置 `UU_VIEWER_VIDEO_DIAGNOSTICS=1` 可开启诊断。测试包会自动开启，并保留原生手柄及震动。
当前安装包使用外部 FFmpeg 解码；不改变帧率请求、解码参数或 Windows 显示配置。
诊断结果尚需 Steam Deck 实机采集，不能据此宣称问题已修复或支持 144 FPS。

| Field / 字段 | Meaning / 含义 |
| --- | --- |
| 接收 / Recv (`rx_au`) | Complete depacketized video access units before the bounded queue; excludes incomplete RTP frames. 解包完成、尚未入队的完整视频访问单元。 |
| `fed_au` | Units passed to the decoder after queue and keyframe gating. 实际交给解码器的访问单元。 |
| 解码 / Decode (`rgb`) | Complete RGB frames read from FFmpeg, before decoded-frame queue drops. 从 FFmpeg 完整读出的 RGB 帧。 |
| 显示 / Show (`shown`) | Video frames successfully submitted to the SDL presenter; excludes UI-only redraws, does not prove physical scanout. 成功提交到显示函数的视频帧，不是硬件扫描输出计数。 |
| 请求 / Request | Selected target FPS in the viewer menu. 菜单选定的目标值。 |
| 主机回报 / Host report | Latest positive FPS value from CONTROL Message 25 field 1; `--` if not supplied. This is a reported setting, not measured encoder output. 主机回报的设定值，不是实测编码帧率。 |
| `au_drop`, `rgb_drop`, `drain_drop`, `gate_drop` | Compressed queue overflow, RGB queue overflow, old decoded frames skipped by drain, keyframe-gate rejection. 各阶段丢弃计数。 |
| `pipe_avg_us`, `pipe_max_us` | Time spent synchronously writing/flushing an access unit into FFmpeg. Includes pipe backpressure; NOT FFmpeg decode duration. 同步管道写入耗时，不等于解码耗时。 |
| `draw_avg_us`, `draw_max_us` | Texture creation/upload, picture/UI rendering and SDL present. 包含纹理上传和 SDL 显示调用的耗时。 |
| `tick_gap_max_us` | Longest interval between event-loop tick handlers in this sample. 事件循环两次处理间的最大间隔。 |
| `errors`, `resets`, `pli` | Decoder errors, consumed local reset flags, depacketizer loss/recovery requests. 解码及重同步计数。 |

Every sample includes `interval_ms`; log counts are interval totals, not assumed
one-second rates. The HUD divides by elapsed time. Ignore startup samples and use
at least 20 seconds of a moving game scene with one remote desktop connected.
Each desktop viewer is a separate process. Do not mix several simultaneous viewers.

请连接一台电脑，在游戏持续运动时采集至少 20 秒；忽略启动初期采样。
日志每条记录自上次采样以来的总数，面板按实际时间换算帧率。
若接收约 60、解码/显示约 30，结合丢帧及耗时定位本地处理；若接收也约 30，
继续检查主机回报、采集/编码以及解包情况，不能仅据此断言 Windows 编码器锁帧。

Only numeric allowlisted fields are emitted with the `DECKPAD_VIDEO_` prefix.
No desktop frames, input, clipboard data, account IDs or authentication material
are recorded by this feature. Existing upstream stderr must still be filtered by
the package launcher; do not publish unfiltered upstream logs.

The publication handoff/main branch is unchanged. This branch is for diagnosis.
