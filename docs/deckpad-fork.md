# DeckPad fork: provenance, changes and validation

## 来源与 Codex 编辑说明 / Attribution

- Upstream: [VallierDev/uu-remote-linux](https://github.com/VallierDev/uu-remote-linux).
- Base: [`6bf05a6361e58dbeec5846f044d234bed04fcf02`](https://github.com/VallierDev/uu-remote-linux/commit/6bf05a6361e58dbeec5846f044d234bed04fcf02), upstream package version 0.2.41.
- Fork owner / requirements and hardware testing: [azuredreamblu-design](https://github.com/azuredreamblu-design).
- OpenAI Codex assisted with protocol analysis, code changes, tests, launch scripts and documentation. AI assistance is disclosed; it is not an OpenAI endorsement, a security audit, or a guarantee of compatibility.
- 原项目架构和原有实现归上游作者及贡献者；本 fork 的新增手柄/震动改动由 Codex 协助完成，用户提供需求并实机测试。保留原提交历史、署名和 GPL-3.0-or-later 许可。

## Changes

- Native gamepad input in the existing SDL viewer and desktop CONTROL channel.
- Ordinary two-motor rumble from official `gpd_vibration` feedback.
- Focus/release gating, pause toggle, controller discovery, neutral/disconnect cleanup.
- Peer-local bounded rumble inbox, finite SDL effects, stop on pause/focus loss/detected disconnection.
- Native protocol/range/state tests and virtual WebRTC input/feedback alongside file RPCs.
- Portable Steam launcher and one-time Windows XInput rumble test under `contrib/deckpad`.
- Batched video decoding, desktop startup and full capture-profile fixes, opt-in numeric diagnostics and a filtered report collector; see [video acceptance](steamdeck-video-acceptance.md).

The earlier custom Python receiver, reverse SSH tunnel and shared port-mapping experiment are excluded from this fork. The native Rust input and rumble implementations are carried from the tested v0.5.1 package; the repository launcher is adapted to source builds and XDG state paths.

No NetEase executable, driver, logo or newly captured screenshot is added. No account secrets, private LAN address, pairing key, raw input report or private session log is included. Existing upstream files and credits remain intact.

## Validation

**User-reported, 2026-10-31:** one Windows + Steam Deck pair accepted native input, phone-hotspot WAN, normal exit/display restoration, reconnect and rumble. Display restoration has a small delay. The user confirmed that the relatively light test pulses were acceptable. These reports do not cover every SteamOS/UU release, game, or controller.

**Original v0.5.1 local checks:** 195 release-viewer tests passed; one pre-existing test was ignored and two Unix IPC tests were excluded because the local sandbox disallowed those sockets. Python migration-installer tests: four passed; release build/help and shell syntax passed. These installer tests apply to the prior downloadable package, not a new general installer in this repository.

For this cleaned fork, run the viewer test suite and build below. GitHub CI should run the unfiltered tests in its standard environment; do not turn local exclusions into permanent CI skips.

**Clean fork local checks:** 193 release-viewer tests passed, one pre-existing ignored, the same two Unix IPC tests excluded. Removing the unused shared-port-map experiment also removed its two dedicated tests. Launcher smoke testing passed with a synthetic viewer: custom binary path, XDG state path, startup and clean exit; Python and shell syntax were checked. No new physical hardware test was performed for the adapted launcher.

```bash
cargo test --locked --features native-viewer,viewer-sdl --bin uuremote-viewer
cargo build --locked --release --features native-viewer,viewer-sdl --bin uuremote-viewer
```

Hardware acceptance and cloud unit tests are different evidence. Suspend/resume, forced process termination, every network-loss pattern, controller replacement, multiple controller slots and broad game compatibility remain incomplete. No upstream merge or official protocol support is claimed.
