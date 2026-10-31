import contextlib
import io
from pathlib import Path
import sys
import tempfile
import unittest
sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from uu_session import diagnostic
from collect_diagnostics import collect

class PrivacyTests(unittest.TestCase):
    def test_report_does_not_mix_reconnected_desktop_children(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "session.log"
            path.write_text("t START uu_session version=test\n"
                            "t DECKPAD_VIDEO_RENDERER display_hz=60\n"
                            "t DECKPAD_VIDEO_SESSION event=1 elapsed_ms=0\n"
                            "t DECKPAD_VIDEO_SAMPLE elapsed_ms=29000 rx_au=144\n"
                            "t DECKPAD_VIDEO_SESSION event=3 reason=1 elapsed_ms=30000\n"
                            "t DECKPAD_VIDEO_RENDERER display_hz=90\n"
                            "t DECKPAD_VIDEO_SESSION event=1 elapsed_ms=0\n"
                            "t DECKPAD_VIDEO_SAMPLE elapsed_ms=1000 rx_au=30\n"
                            "t DECKPAD_VIDEO_SESSION event=3 reason=2 elapsed_ms=2000\n")
            output=io.StringIO()
            with contextlib.redirect_stdout(output):collect(path)
            text=output.getvalue()
            self.assertIn("display_hz=90",text)
            self.assertIn("rx_au=30",text)
            self.assertIn("event=3 reason=2",text)
            self.assertNotIn("display_hz=60",text)
            self.assertNotIn("rx_au=144",text)
            self.assertNotIn("reason=1",text)
    def test_numeric_events_only(self):
        safe = b"DECKPAD_VIDEO_SAMPLE interval_ms=1000 rx_au=60 rgb=30 shown=30"
        self.assertEqual(diagnostic(safe), safe.decode())
        for bad in [b"DECKPAD_VIDEO_SAMPLE token=secret", b"DECKPAD_VIDEO_SYNC fps=60 token=abc",
                    b"DECKPAD_VIDEO_SYNC fps=60\nextra", b"DECKPAD_VIDEO_RAW payload=123",
                    b"DECKPAD_VIDEO_SYNC fps=60 path=/tmp/credential"]:
            self.assertIsNone(diagnostic(bad))
    def test_report_is_latest_session_and_allowlisted(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "session.log"
            path.write_text("t START uu_session version=old\nt DECKPAD_VIDEO_SAMPLE shown=99\n"
                            "t START uu_session version=new\nt credential=secret\n"
                            "t DECKPAD_VIDEO_SYNC fps=60 chroma=1\nt DECKPAD_VIDEO_SAMPLE shown=30\n"
                            "t DECKPAD_VIDEO_SYNC token=secret\n")
            output = io.StringIO()
            with contextlib.redirect_stdout(output):
                collect(path)
            self.assertIn("shown=30", output.getvalue())
            self.assertNotIn("99", output.getvalue())
            self.assertNotIn("secret", output.getvalue())
    def test_network_samples_do_not_evict_settings_timeline(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "session.log"
            lines = ["t START uu_session version=test", "t DECKPAD_VIDEO_REQUEST quality=3 request_id=123 elapsed_ms=0"]
            for i in range(60):
                lines.extend([f"t DECKPAD_VIDEO_SAMPLE elapsed_ms={i} rx_au=30",
                              f"t DECKPAD_VIDEO_NETWORK elapsed_ms={i} rtp_frames=30 seq_gaps=0"])
            lines.append("t DECKPAD_VIDEO_NETWORK payload=private")
            path.write_text("\n".join(lines))
            output = io.StringIO()
            with contextlib.redirect_stdout(output):
                collect(path)
            text = output.getvalue()
            self.assertIn("quality=3 request_id=123", text)
            self.assertEqual(text.count("DECKPAD_VIDEO_SAMPLE "), 40)
            self.assertEqual(text.count("DECKPAD_VIDEO_NETWORK "), 40)
            self.assertNotIn("private", text)

if __name__ == "__main__":
    unittest.main()
