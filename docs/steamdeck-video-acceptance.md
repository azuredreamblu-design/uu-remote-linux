# Steam Deck video fixes and acceptance / 视频修复与验收

## Changes / 改动

- Batch compressed video units in FIFO order, independently of presentation.
  The former one-unit-per-8-ms-loop path could overflow at 144 units/s and
  repeatedly enter keyframe recovery. The bounded queue is now eight units;
  each pump handles up to eight units or a four-millisecond work budget.
- Establish the interactive desktop only with a connected peer, open CONTROL,
  and either a matching echo or a decoded picture. Once established, the
  30-second startup deadline no longer applies. Echo/capability state is not
  fabricated, and other tools retain their negotiation requirements.
- Restore saved FPS/quality/chroma together and send all three known fields
  when any of them changes. Ordinary profiles add no monitor-mode fields.
- Add opt-in numeric receive/decode/presentation, RTP and settings diagnostics.
  The source Steam launcher filters these markers and the collector isolates
  the latest sequential desktop child. It does not isolate simultaneous viewers.

压缩帧按顺序批量处理，避免处理节奏低于输入速率导致队列溢出和反复等待关键帧。
桌面已有真实连接、控制通道及解码画面时不再因缺少 echo 在 30 秒主动退出。
帧率、画质、色度一起发送，避免保存设置未恢复或请求省略帧率。
诊断默认关闭，开启后只输出允许的数值和固定状态标记。

## Automated checks / 自动检查

The tested Rust implementation is commit
`b643d7b8b9c091153f791d40eb69d05b84f2e6d2` in the local development history.
The release viewer suite reported **203 passed, 0 failed, 1 pre-existing ignored,
2 filtered**. The same two Unix IPC tests were omitted only in that environment's
command; CI has no added exclusions. Release build and binary help passed.

The downloadable v0.3 package had 18 installer/rollback/report tests passing.
For this repository integration, four numeric-privacy and report-segmentation
tests pass under `contrib/deckpad/tests`, and shell syntax passes. The collector
uses the repository launcher's state path; the downloadable migration installer
and its machine-specific layout are not copied into this repository.

```bash
cargo test --locked --features native-viewer,viewer-sdl --bin uuremote-viewer
cargo build --locked --release --features native-viewer,viewer-sdl --bin uuremote-viewer
python3 -m unittest discover -s contrib/deckpad/tests -v
bash -n contrib/deckpad/deck_start.sh
```

## Owner hardware report, 2026-10-31 / 用户实机结果

The owner retested downloadable video-fix v0.3 on one Windows host and one
Steam Deck with a reported 90-Hz display. Its numeric report shows:

- Desktop readiness at 3.238 seconds with `echo=0`, `picture=1`, `control_open=1`.
  This exercises the actual-picture fallback rather than a matching echo.
- Normal return reason 0 at 274.606 seconds, exceeding the former 30-second
  deadline. The reason code alone does not identify who initiated exit.
- Requests for 144 and 90 FPS, followed by a selected 60 FPS at 228.764 seconds.
  Only the last roughly 40 seconds of samples were retained; those samples
  therefore validate the 60-FPS phase, not sustained 90/144 FPS.
- Most retained receive/decode samples near 59–61 pictures/s and zero compressed
  queue drops throughout that retained window.
- One decoder error around 269 seconds, with no new RGB pictures in two
  consecutive sample intervals, followed by recovery. Some additional recovery
  and presentation drops remain. The cause of this interruption is not established.

用户反馈“这次似乎好了”。报告确认约 4 分 35 秒运行并正常结束，切到 60 FPS 后大部分
接收、解码接近目标，保留采样中的压缩队列溢出为零；仍有短暂解码中断后恢复。
不能据此宣布完全稳定，或宣称持续 90/144 FPS 已验收。

Earlier reports briefly measured 139 decoded and 119 submitted pictures/s after
the scheduling fix. Submissions are not physical display scanout. Neither those
samples nor menu requests establish sustained high-rate display performance.
`host_fps=0` means no valid host FPS setting report, not zero received frames.

No raw private session logs, device identifiers or credentials are included.
The Windows multi-monitor restoration issue has not been causally identified or
fixed by these changes. Long sessions, suspend/resume and broader host/controller
compatibility still require acceptance testing.
