# Steam Deck high-rate receive scheduling / 高帧率接收调度

## Observed failure / 已复现的问题

With a 144 FPS target, a Steam Deck diagnostic report showed 143–144 complete
video access units received per second, 18–23 compressed queue drops per second,
roughly 113–125 units rejected by keyframe recovery per second, and only a few
pictures reaching the decoder. There were no decoded-RGB queue drops or reported
decoder-call errors in that sample. Most synchronous FFmpeg writes took tens of
microseconds; rendering generally took about 5–6 ms.

The viewer consumed at most one compressed unit on each 8 ms input tick. That
limits consumption to about 125 units/s regardless of the FFmpeg decoder's actual
capacity. A sustained 144-unit/s stream therefore inevitably overflows the queue.
Every overflow requests recovery, which discards dependent pictures until a key
frame arrives, making the visible frame rate much worse than the initial shortfall.

实机日志确认：144 帧输入超过了原来每 8 毫秒只处理一帧的约 125 帧/秒上限。
压缩队列溢出后，又因等待关键帧丢弃大量帧，形成持续重同步。这解释了这次日志中的
严重掉帧；尚不能证明所有先前的 30 FPS 情况都有完全相同的原因。

## Change / 修正

- Consume compressed access units in FIFO batches, up to eight calls or a 4 ms
  work budget per input tick. A synchronous decoder call can still exceed the
  budget; the diagnostic pipe timings make this visible.
- Present only the newest decoded RGB picture at the end of a tick. Do not skip
  compressed reference pictures merely because presentation is slower.
- Increase the bounded compressed queue from three to eight access units to
  tolerate short UI redraw stalls. Consume immediately; do not wait for the queue
  to fill. Persistent overload still invokes recovery.
- Keep the existing keyframe gate after real loss, input/rumble polling, connection
  cleanup, decoder settings and FPS requests.

按顺序批量送入压缩帧；解码完成后选择最新画面显示；保留真实丢包后的恢复保护。
没有加入硬件解码，也没有宣称 Steam Deck 可显示 144 Hz。

## Validation and limits / 验证与限制

A deterministic arrival schedule models eight seconds at 144 access units/s with
125 input ticks/s. The old one-pop policy overflows even with an eight-slot queue.
The new pump must deliver all 1152 units in order, with no compressed drops or
resets, while presenting at most once per tick. Separate tests cover choosing the
newest RGB picture, polling asynchronous output without new input, and preserving
keyframe recovery after actual compressed loss.

The packaged build retains the diagnostic counters for real-device acceptance.
Expected at 144 FPS input: `fed_au` tracks `rx_au` over time, with `au_drop`,
`gate_drop` and `resets` near zero after startup. Decode and display rates may still
be lower because of hardware or compositor limits; this fix does not establish a
hardware performance guarantee. A 90 Hz panel cannot physically display 144 unique
frames each second. Do not call the issue fully fixed until the owner retests it.

实机验收应关注：启动稳定后接收与送解码计数大致相等，压缩队列溢出和关键帧等待不再
持续增长。显示帧率还受实际解码、渲染和屏幕刷新率影响，须回收新日志确认。

### Subsequent retest / 后续实机复测

The v0.1 scheduling build reached 139 decoded and 119 submitted pictures/s in one
sample. It later received only about 30 complete units/s while the game continued
moving; decode/presentation kept up with those units. The sustained local queue
overflow improved, but the later receive-rate decline remains unresolved. See
[receive-rate diagnostics](steamdeck-video-rate-drop.md). The acceptance criteria
above remain targets, not a claim of sustained success.

v0.1 曾达到解码 139、提交显示 119 帧/秒；随后接收完整帧降至约 30，游戏仍在运动。
旧的持续队列溢出得到改善，后续接收率下降尚未解决；上述标准仍为验收目标。
