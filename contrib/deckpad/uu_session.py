"""Supervise the native viewer; gamepad lives inside its SDL desktop process."""
import fcntl
import os
from pathlib import Path
import subprocess
import sys
import threading
import time

from diagnostics import initialize, note
from lifecycle import stop_desktops, stop_process

ROOT = Path(__file__).resolve().parent
VIEWER = Path(os.environ.get("UU_VIEWER_BIN", str(ROOT.parents[1] / "target/release/uuremote-viewer"))).expanduser().resolve()
STATE = Path(os.environ.get("UU_DECKPAD_STATE_DIR", str(Path.home() / ".local/state/uu-remote-linux/deckpad")))


def diagnostic(line):
    # Exact allowlist only; upstream stderr can contain account or session material.
    text = line.decode("utf-8", errors="replace").strip()
    fixed = {"DECKPAD_NATIVE_ENABLED", "DECKPAD_NATIVE_CONTROLLER_OPEN",
             "DECKPAD_NATIVE_CONNECT_QUEUED", "DECKPAD_NATIVE_RELEASE_QUEUED",
             "DECKPAD_NATIVE_PAUSED", "DECKPAD_NATIVE_RESUMING",
             "DECKPAD_RUMBLE_ENABLED", "DECKPAD_RUMBLE_SUPPORTED", "DECKPAD_RUMBLE_UNAVAILABLE",
             "DECKPAD_RUMBLE_RECEIVED", "DECKPAD_RUMBLE_APPLIED", "DECKPAD_RUMBLE_OUTPUT_FAILED"}
    if text in fixed:
        return text
    return None


def read_status(stream):
    try:
        while line := stream.readline(4096):
            if marker := diagnostic(line):
                note(marker)
    except (OSError, ValueError):
        pass
    finally:
        stream.close()


def run(viewer=VIEWER):
    lock_path = STATE / "session.lock"
    lock_path.parent.mkdir(parents=True, exist_ok=True)
    lock = lock_path.open("a")
    try:
        fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
    except BlockingIOError:
        note("ALREADY_RUNNING: exit the old UU+Gamepad session first")
        lock.close()
        return 0
    viewer_child = reader = None
    try:
        if not os.environ.get("DISPLAY"):
            raise RuntimeError("Start deck_start.sh from Steam on the Deck, not from SSH")
        if not viewer.is_file():
            raise RuntimeError("Viewer not found; build the viewer or set UU_VIEWER_BIN / 未找到控制端，请先编译或设置 UU_VIEWER_BIN")
        env = os.environ.copy()
        env.update(UU_LANG="zh", UU_VIEWER_CODEC="h264", SDL_VIDEODRIVER="x11",
                   GDK_BACKEND="x11", SDL_JOYSTICK_ALLOW_BACKGROUND_EVENTS="1",
                   UU_VIEWER_NATIVE_GAMEPAD="1")
        viewer_child = subprocess.Popen([str(viewer), "ui"], cwd=viewer.parent,
            env=env, start_new_session=True, stdin=subprocess.DEVNULL,
            stdout=subprocess.DEVNULL, stderr=subprocess.PIPE)
        reader = threading.Thread(target=read_status, args=(viewer_child.stderr,), daemon=True)
        reader.start()
        note("SESSION_RUNNING transport=UU_NATIVE_GAMEPAD_EXPERIMENTAL; select one Windows desktop")
        while viewer_child.poll() is None:
            time.sleep(0.2)
        note(f"VIEWER_EXIT status={viewer_child.returncode}")
        return viewer_child.returncode
    finally:
        if viewer_child:
            stop_desktops(viewer_child.pid, viewer)
            stop_process(viewer_child)
        if reader:
            reader.join(timeout=1)
        lock.close()
        note("SESSION_STOPPED")


if __name__ == "__main__":
    initialize("uu_session")
    try:
        sys.exit(run())
    except KeyboardInterrupt:
        pass
    except Exception as exc:
        note("ERROR " + type(exc).__name__ + ": " + str(exc))
        sys.exit(1)
