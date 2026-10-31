# Receive-rate decline diagnostics / 接收速率下降诊断

## Retest evidence / 实机结果

After the batch scheduling fix, the owner's report included 143 complete access
units received, 139 fed/decoded pictures and 119 presentation submissions in one
second. Later samples stabilized at approximately 30 received, fed, decoded and
presented pictures/s, with zero compressed queue drops in most samples. The owner
confirmed that the game was continuously moving even after the decline. Some
recovery events remain. This is evidence that the sustained 125-unit/s queue
overflow was addressed, not that all playback problems were solved.

修复后曾收到约 143 帧/秒、解码 139 帧/秒、提交显示 119 帧/秒。随后完整帧接收率
本身降到约 30，解码与显示跟上接收率；用户确认游戏仍在持续运动。仍有少量恢复事件，
不能宣布全部修复。提交显示次数不是屏幕实际扫描出的独立画面数。

The existing RX counter is after depacketization. It alone cannot prove that the
host encoder switched to 30 FPS. Do not invent a host adaptive-rate policy or
disable loss recovery based on this report.

现有 rx_au 位于组帧之后，不能独立证明主机编码器主动切成 30 FPS。本次补充诊断，
不改变帧率请求、网络反馈、丢包恢复、显示器模式、解码器或手柄协议。

## New numeric observations / 新增数值

With `UU_VIEWER_VIDEO_DIAGNOSTICS=1`, each one-second sample has a matching
`DECKPAD_VIDEO_NETWORK` line sharing `elapsed_ms`:

- `rtp_packets`, `rtp_bytes`: video packets and payload bytes before depacketization;
  includes duplicates/late packets, excludes audio, contains no payload data.
- `rtp_frames`: forward timestamp changes observed on forward sequence progress;
  first packet counts as one frame. This is not a host-side frame count and can
  undercount reordered frames. No absolute sequence number or timestamp is logged.
- `rtp_markers`: packets with the RTP marker bit, including late/duplicate markers.
- `seq_gaps`: holes observed when the sequence advances; not final loss.
- `late_packets`: old or duplicate packets; not necessarily lost pictures.
- `timestamp_steps`, `timestamp_ticks`, `rtp_timing_fps`: forward timestamp changes,
  total corresponding 90 kHz ticks and their ratio. This measures cadence in the
  received stream, not acknowledgement of the configured host FPS. Missing frames,
  timestamp discontinuities and multiple video tracks can bias it.

SAMPLE now records `elapsed_ms` and the locally selected `requested_quality`.
REQUEST records FPS, quality, chroma and follow-remote changes with an opaque
numeric request ID, elapsed time and send success. Send success means accepted by
the local data channel, not applied by the host. Known capture replies include
the request ID and error-field count. `host_fps=0` still means unknown.

用接收前后的帧数判断下降发生在哪层；将设置请求和下降时间对齐。`seq_gaps` 可能被
随后到达的包修复，不能当作最终丢包率。所有记录只含计数、数值设置和布尔值。

## Reproduce and interpret / 复测与判断

Use the same continuously moving game, FPS 144 and unchanged quality/chroma for
the first run. Do not change settings after video starts. When it falls, keep
moving for another five seconds and export the numeric report promptly. If no
decline occurs, export after 30 seconds. Keep the same Steam launch entry.

先保持原画质、真彩与 144 FPS，开始显示后不再改选项；持续运动，掉到 30 后再运行
5 秒并立即导出报告。没有掉帧则 30 秒后导出。诊断包仍沿用原 Steam 入口。

- RTP frame progress near 144/s but complete RX near 30/s points toward assembly
  or missing fragments. Inspect sequence holes and recovery counts.
- Both RTP frame progress and complete RX near 30/s, with clean sequences and
  timestamp cadence near 30, support lower incoming stream cadence. This still
  does not identify why the host chose it; compare settings events and host-side
  capture/encoder evidence next.
- RX near 144/s but decoded/shown much lower points back into the local pipeline.

## Validation / 验证

Tests exercise a synthetic 144-to-30 timestamp cadence transition without any
decoder, multi-packet pictures, sequence/timestamp wrap, observed gaps, late
arrival and duplicate markers. The previous scheduling/recovery regression tests
remain required. Actual host-decline cause and this diagnostic build's hardware
results remain unverified until the next owner report.
