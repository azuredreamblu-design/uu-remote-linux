# Desktop startup and capture profiles / 桌面启动与画质请求

## Evidence / 证据

The owner's v0.2 report contains two desktop timelines within one launcher run.
The later timeline shows healthy video at about 60 received/decoded pictures/s,
then ends around elapsed_ms=30001 with the UI reporting “official host interop is
not finished”. That wording is the local translation of `CONTROL echo timed out`.
The SDL loop had an unconditional 30-second deadline gated only by a matching
echo reply. It ignored already decoded desktop pictures and an open connected
control channel.

v0.2 日志显示视频已正常接收、解码，却在约 30 秒出现“官方主机互通尚未完成”。
这句话是客户端对 `CONTROL echo timed out` 的本地翻译；旧代码只认可 echo 回应，
即使已经显示桌面，也会因未识别到 echo 而主动退出。

The same echo gate prevented restoring saved preferences. `requested_fps=144`
was the local menu state, not evidence of a sent request or host acknowledgement.
At 19.166 seconds a quality-only request (field 2 = 4, Original) was sent; the
incoming RTP cadence then changed from about 60 to 30/s with clean sequence
progress. This supports testing a complete profile to avoid relying on the host
preserving omitted FPS. It does not establish the proprietary host's internal
implementation. Existing protocol fields are documented in `capture_setting.rs`.

同一个 echo 门槛还阻止了保存设置的恢复。日志里的 requested_fps=144 只是本地选项。
19.166 秒发送仅含 quality=4 的请求后，接收到的 RTP 帧节奏降为约 30，序号基本连续。
本次让请求携带完整帧率/画质/色度，避免依赖主机保留省略的字段；实机效果仍待确认。

## Changes / 改动

- Only for the interactive SDL desktop, usable startup requires a connected
  peer, open CONTROL channel, and either a correlated echo or a decoded picture
  from that desktop. Plain media packet counts do not qualify.
- A latched startup guard restores preferences once and stops applying the
  30-second startup timeout after establishment. A silent/unusable startup still
  times out. Later peer/signaling termination keeps the existing cleanup path.
- Normal desktop input, gamepad and heartbeat use the same desktop readiness.
  Echo flags are never fabricated; file tools, clipboard capability negotiation,
  power commands and secure typing retain their separate requirements.
- Restore one complete FPS/quality/chroma profile, including default values.
  Every subsequent quality/FPS/chroma change sends those known fields together.
  Ordinary profile requests contain no display size or topology fields. Explicit
  resolution changes carry their previous fields alongside the profile.
- Replace the misleading generic interop error text with a specific timeout.
- Numeric SESSION markers distinguish desktop children and bounded exit reasons.
  The report collector keeps only the latest child in the current launcher run.
  The launcher forwards final diagnostic markers without treating them as errors.

仅调整桌面就绪判定、启动超时、已知画质请求组合和诊断分段；保持现有认证、传输、
退出清理和其他工具的功能协商。不会伪造 echo 或宣称主机已确认设置。

## Diagnostics / 诊断

`DECKPAD_VIDEO_SESSION event=1` starts a desktop report. Event 2 records readiness
(`echo`, `picture`, `control_open`). Event 3 closes it with a numeric reason:
0 normal, 1 desktop startup timeout, 2 signaling room closed, 3 remote released,
4 other error. No raw exception payload, desktop content or identifiers are logged.
Capture REQUEST lines contain the complete selected profile; `sent=1` still
means the local channel accepted the message, not that the host applied it.

## Validation / 验证

Regression tests replay a picture starting at 8 seconds with no echo, then keep
the startup guard alive through 120 seconds while restoring preferences once.
A no-ready case must still fail at 30 seconds. Readiness tests reject unopened
CONTROL, disconnected peers and packet counts without pictures/echo. Wire tests
assert explicit FPS survives quality/chroma changes without unsolicited display
size fields. Collector tests isolate repeated desktop children and their final
reason markers. Actual Windows-host FPS preservation and >30-second operation
remain pending owner retest.

复测：保持 144 FPS，持续运行至少 60 秒，再切一次画质，继续 10 秒后导出报告。
如中途断开，直接导出同一份报告；SESSION 结束码用于进一步定位。
