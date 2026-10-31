#!/bin/bash
set -eu
script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
export UU_DECKPAD_STATE_DIR="${UU_DECKPAD_STATE_DIR:-${XDG_STATE_HOME:-$HOME/.local/state}/uu-remote-linux/deckpad}"
umask 077
mkdir -p -- "$UU_DECKPAD_STATE_DIR"
exec >>"$UU_DECKPAD_STATE_DIR/session.log" 2>&1
printf '\nDECKPAD_LAUNCH %s\n' "$(date -Iseconds)"
exec python3 -u -X faulthandler "$script_dir/uu_session.py" "$@"
