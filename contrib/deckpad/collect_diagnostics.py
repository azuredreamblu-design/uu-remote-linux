"""Print only allowlisted numeric diagnostics from the most recent session."""
from pathlib import Path
from uu_session import STATE, diagnostic


def collect(path=STATE / "session.log"):
    if not path.is_file():
        print("NO_SESSION_LOG: launch the existing Steam entry and connect first")
        return
    with path.open("rb") as stream:
        stream.seek(max(0, path.stat().st_size - 2 * 1024 * 1024))
        lines = stream.read().splitlines()
    # Do not mix a prior connection's rates with the current launch.
    start = next((i for i in range(len(lines)-1, -1, -1)
                  if b"START uu_session version=" in lines[i]), 0)
    safe = []
    for line in lines[start:]:
        parts = line.split(b" DECKPAD_VIDEO_", 1)
        if len(parts) != 2:
            continue
        marker = diagnostic(b"DECKPAD_VIDEO_" + parts[1])
        if marker:
            safe.append(marker)
    # The GTK launcher can start several desktop children within one supervisor launch.
    # Keep the latest child's timeline, rather than mixing reset elapsed_ms counters.
    session = next((i for i in range(len(safe)-1, -1, -1)
                    if safe[i].startswith("DECKPAD_VIDEO_SESSION event=1 ")), None)
    if session is not None:
        renderer = next((line for line in reversed(safe[:session])
                         if line.startswith("DECKPAD_VIDEO_RENDERER ")), None)
        safe = ([renderer] if renderer else []) + safe[session:]
    print("DECKPAD_VIDEO_REPORT version=3")
    # Keep paired pipeline/packet samples; packet lines must not evict settings events.
    samples = [i for i, line in enumerate(safe) if line.startswith("DECKPAD_VIDEO_SAMPLE ")]
    keep = set(samples[-40:])
    network = [i for i, line in enumerate(safe) if line.startswith("DECKPAD_VIDEO_NETWORK ")]
    keep.update(network[-40:])
    events = [i for i, line in enumerate(safe) if not line.startswith(("DECKPAD_VIDEO_SAMPLE ", "DECKPAD_VIDEO_NETWORK "))]
    keep.update(events[-25:])
    for i in sorted(keep):
        print(safe[i])
    if not safe:
        print("NO_VIDEO_SAMPLES: confirm this video fix build is launched and connected")

if __name__ == "__main__":
    collect()
