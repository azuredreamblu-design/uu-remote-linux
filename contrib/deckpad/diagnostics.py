"""Append-only process logs. Never record pairing material or input reports."""
import atexit
import faulthandler
import os
import signal
import sys
from datetime import datetime
from pathlib import Path

VERSION = "0.5.1-native-rumble-experimental"


def note(message):
    print(datetime.now().astimezone().isoformat(timespec="seconds") + " " + message, flush=True)


class Tee:
    def __init__(self, console, logfile):
        self.console, self.logfile = console, logfile

    def write(self, text):
        self.logfile.write(text)
        self.logfile.flush()
        try:
            self.console.write(text)
            self.console.flush()
        except (OSError, ValueError):
            pass
        return len(text)

    def flush(self):
        self.logfile.flush()
        try:
            self.console.flush()
        except (OSError, ValueError):
            pass

    def fileno(self):
        return self.logfile.fileno()


def initialize(role, logfile=None):
    if logfile:
        # Append; retain evidence even after a restart or a closed console.
        f = Path(logfile).open("a", encoding="utf-8", buffering=1)
        sys.stdout, sys.stderr = Tee(sys.stdout, f), Tee(sys.stderr, f)
        faulthandler.enable(file=f, all_threads=True)
    else:
        faulthandler.enable(all_threads=True)
    note(f"START {role} version={VERSION} pid={os.getpid()}")
    atexit.register(lambda: note(f"PROCESS_EXIT {role}"))

    def terminated(signum, frame):
        note(f"SIGNAL {signal.Signals(signum).name}; exiting")
        raise SystemExit(128 + signum)

    for name in ("SIGTERM", "SIGHUP"):
        sig = getattr(signal, name, None)
        if sig is not None:
            signal.signal(sig, terminated)
