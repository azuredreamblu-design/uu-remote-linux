"""Track owned desktop processes and shut them down gracefully. No cloud joins."""
from dataclasses import dataclass
import os
from pathlib import Path
import signal
import subprocess
import time

from diagnostics import note

@dataclass(frozen=True)
class Desktop:
    pid: int
    started: str
    device: str


def identity(pid):
    try:
        raw = Path(f"/proc/{pid}/stat").read_text()
        parts = raw[raw.rfind(")") + 2:].split()
        return None if parts[0] == "Z" else parts[19]
    except (OSError, IndexError):
        return None


def command_target(args):
    if len(args) not in (7, 8) or args[1] != "connect":
        return None
    if args[3:7] != ["false", "-", "echo", "legacy"]:
        return None
    if len(args) == 8 and args[7] != "--view-only":
        return None
    device = args[2]
    return device if device and len(device) <= 256 and not device.startswith("-") else None


def desktops(group, viewer):
    result = []
    for entry in Path("/proc").iterdir():
        if not entry.name.isdigit():
            continue
        pid = int(entry.name)
        try:
            if entry.stat().st_uid != os.getuid() or os.getpgid(pid) != group:
                continue
            if (entry / "exe").resolve() != viewer.resolve():
                continue
            args = [part.decode("utf-8") for part in (entry / "cmdline").read_bytes().split(b"\0") if part]
            device = command_target(args)
            started = identity(pid)
            if device and started:
                result.append(Desktop(pid, started, device))
        except (OSError, ValueError, UnicodeError):
            continue
    return result


def stop_process(child, timeout=8):
    """Allow the actual root its full cleanup interval before removing descendants."""
    if child is None:
        return
    if child.poll() is None:
        try:
            child.send_signal(signal.SIGTERM)
        except ProcessLookupError:
            pass
        try:
            child.wait(timeout=timeout)
        except subprocess.TimeoutExpired:
            note("PROCESS_CLEANUP_TIMEOUT; forcing owned group")
    try:
        os.killpg(child.pid, signal.SIGKILL)
    except ProcessLookupError:
        pass
    child.wait(timeout=2)


def stop_desktops(group, viewer, timeout=8):
    # Leave GTK and decoder running until the desktop closes its network session.
    current = desktops(group, viewer)
    for item in current:
        if identity(item.pid) == item.started:
            try:
                os.kill(item.pid, signal.SIGTERM)
            except ProcessLookupError:
                pass
    deadline = time.monotonic() + timeout
    while any(identity(item.pid) == item.started for item in current):
        if time.monotonic() >= deadline:
            note("DESKTOP_CLEANUP_TIMEOUT")
            break
        time.sleep(0.05)

