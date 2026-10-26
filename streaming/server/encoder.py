"""Encoder process for the stream (K7). The encoder runs as its own `gst-launch-1.0` process and
sends RTP over local UDP to the WebRTC server. x264 is GPL, so it stays in a separate process
(file 39 rule 3); NVENC (`nvh264enc`) uses the same boundary.

Encoder choice: `auto` uses NVENC only if a real test encode succeeds, otherwise x264 (software).
Encode latency comes from GStreamer's latency tracer (per-buffer time inside the encoder element).
"""
from __future__ import annotations

import re
import shutil
import statistics
import subprocess

# Browsers need H.264 constrained baseline with packetization-mode 1.
RTP_CAPS = ("application/x-rtp,media=video,clock-rate=90000,encoding-name=H264,payload=96,"
            "packetization-mode=(string)1,profile-level-id=(string)42e01f")

ENCODERS = {
    # name: (element chain, element factory name used for the tracer)
    "x264": ("videoconvert ! video/x-raw,format=I420 ! x264enc name=enc tune=zerolatency speed-preset=ultrafast "
             "bitrate={kbps} key-int-max={gop} bframes=0 ! video/x-h264,profile=constrained-baseline", "enc"),
    "nvh264": ("videoconvert ! video/x-raw,format=NV12 ! nvh264enc name=enc preset=low-latency-hq zerolatency=true rc-mode=cbr "
               "bitrate={kbps} gop-size={gop} bframes=0 ! video/x-h264,profile=constrained-baseline", "enc"),
}


def probe_nvenc(timeout_s: float = 20.0) -> tuple[bool, str]:
    """True only if nvh264enc exists and encodes ten test frames."""
    if not shutil.which("gst-inspect-1.0") or subprocess.run(["gst-inspect-1.0", "nvh264enc"], capture_output=True).returncode != 0:
        return False, "nvh264enc element not available"
    r = subprocess.run(["gst-launch-1.0", "-q", "videotestsrc", "num-buffers=10", "!", "video/x-raw,width=640,height=360",
                        "!", "videoconvert", "!", "nvh264enc", "!", "fakesink"], capture_output=True, text=True, timeout=timeout_s)
    return (r.returncode == 0, "test encode ok" if r.returncode == 0 else f"test encode failed: {(r.stderr or r.stdout)[-300:]}")


def choose(requested: str) -> tuple[str, str]:
    """('x264' | 'nvh264', reason)."""
    if requested == "x264":
        return "x264", "requested"
    ok, why = probe_nvenc()
    if requested == "nvh264":
        if not ok:
            raise RuntimeError(f"NVENC requested but unavailable: {why}")
        return "nvh264", why
    return ("nvh264", why) if ok else ("x264", f"software fallback ({why})")


def pipeline(encoder: str, display: str, width: int, height: int, fps: int, kbps: int, port: int) -> str:
    chain, _ = ENCODERS[encoder]
    return (f"ximagesrc display-name={display} use-damage=false show-pointer=false ! "
            f"video/x-raw,framerate={fps}/1 ! videoscale ! video/x-raw,width={width},height={height} ! queue max-size-buffers=2 leaky=downstream ! "
            + chain.format(kbps=kbps, gop=fps) +
            f" ! rtph264pay config-interval=-1 pt=96 aggregate-mode=zero-latency mtu=1200 ! udpsink host=127.0.0.1 port={port} sync=false async=false")


TRACER_ENV = {"GST_TRACERS": "latency(flags=element)", "GST_DEBUG": "GST_TRACER:7", "GST_DEBUG_NO_COLOR": "1"}
_LAT = re.compile(r"element-latency.*?element=\(string\)(\w+).*?time=\(guint64\)(\d+)")


def encode_latency_ms(log_text: str, element: str = "enc") -> dict:
    """Per-buffer latency of the encoder element from the latency tracer log: count, mean, p50, p95 (ms)."""
    v = [int(m.group(2)) / 1e6 for m in _LAT.finditer(log_text) if m.group(1) == element]
    if not v:
        return {"count": 0}
    v.sort()
    return {"count": len(v), "mean": round(statistics.fmean(v), 3), "p50": round(v[len(v) // 2], 3), "p95": round(v[min(len(v) - 1, int(len(v) * 0.95))], 3)}
