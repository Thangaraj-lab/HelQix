"""K7 pure-logic tests (no GStreamer, no browser): session guards, encoder pipeline text, tracer parsing."""
import sys
from pathlib import Path

import pytest

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "server"))
from encoder import RTP_CAPS, choose, encode_latency_ms, pipeline  # noqa: E402
from guard import SessionGuard  # noqa: E402


class Clock:
    def __init__(self):
        self.t = 100.0

    def __call__(self):
        return self.t


def test_idle_timeout_and_touch():
    c = Clock()
    g = SessionGuard(idle_timeout_s=10, max_session_s=100, clock=c)
    c.t += 9
    assert g.state()[0] == "ok"
    g.touch()
    c.t += 9
    assert g.state() == ("ok", pytest.approx(1.0))
    c.t += 1.5
    assert g.state() == ("idle", 0.0)


def test_hard_cap_wins_even_with_constant_input():
    c = Clock()
    g = SessionGuard(idle_timeout_s=10, max_session_s=30, clock=c)
    for _ in range(31):
        c.t += 1
        g.touch()
    assert g.state() == ("hard-cap", 0.0)


def test_limits_must_be_positive():
    with pytest.raises(ValueError):
        SessionGuard(idle_timeout_s=0, max_session_s=10)


def test_pipelines_keep_the_encoder_in_its_own_chain_and_send_rtp_locally():
    sw = pipeline("x264", ":99", 1280, 720, 30, 4000, 5004)
    assert "x264enc name=enc" in sw and "tune=zerolatency" in sw and "bframes=0" in sw
    assert sw.startswith("ximagesrc display-name=:99") and sw.endswith("udpsink host=127.0.0.1 port=5004 sync=false async=false")
    assert "constrained-baseline" in sw and "key-int-max=30" in sw
    hw = pipeline("nvh264", ":99", 1280, 720, 30, 4000, 5004)
    assert "nvh264enc name=enc" in hw and "gop-size=30" in hw
    assert "packetization-mode=(string)1" in RTP_CAPS and "42e01f" in RTP_CAPS


def test_requesting_x264_never_probes_and_nvenc_must_really_work(monkeypatch):
    import encoder
    monkeypatch.setattr(encoder, "probe_nvenc", lambda: (False, "no nvidia"))
    assert choose("x264") == ("x264", "requested")
    assert choose("auto")[0] == "x264" and "software fallback" in choose("auto")[1]
    with pytest.raises(RuntimeError):
        choose("nvh264")
    monkeypatch.setattr(encoder, "probe_nvenc", lambda: (True, "test encode ok"))
    assert choose("auto") == ("nvh264", "test encode ok")


def test_tracer_log_parsing():
    log = "\n".join([
        "0:00:01.0 ... GST_TRACER :0:: element-latency, element-id=(string)0x1, element=(string)enc, src=(string)src, time=(guint64)4000000, ts=(guint64)1;",
        "0:00:01.1 ... GST_TRACER :0:: element-latency, element-id=(string)0x1, element=(string)enc, src=(string)src, time=(guint64)6000000, ts=(guint64)2;",
        "0:00:01.2 ... GST_TRACER :0:: element-latency, element-id=(string)0x2, element=(string)pay, src=(string)src, time=(guint64)1000000, ts=(guint64)3;",
    ])
    s = encode_latency_ms(log)
    assert s["count"] == 2 and s["mean"] == 5.0 and s["p95"] == 6.0
    assert encode_latency_ms("nothing") == {"count": 0}
