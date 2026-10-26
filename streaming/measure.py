"""K7 measurement driver. Runs inside the stream container (needs NET_ADMIN for tc netem):

  for RTT 0, 40, 80 ms (netem on loopback) and RTT 0 through a local TURN relay:
      start a headless Chrome viewer -> n latency samples + WebRTC stats; collect server stats
  then: idle-timeout shutdown and hard-cap shutdown checks.

Writes <out>/results.json and <out>/results.md. Exit 1 if the mechanics fail (no stream, no samples,
a guard does not fire, the wrong encoder ran). Latency is reported, never failed: go/no-go against
file 38 (T2 under 100 ms at RTT 40 ms) is a judgement for the founder, and a software (x264) run is
not a GPU result.
Usage: python3 measure.py --out /out [--n 40] [--encoder auto|x264|nvh264] [--no-turn]
"""
from __future__ import annotations

import argparse
import asyncio
import json
import os
import platform
import secrets
import shutil
import subprocess
import sys
import threading
import time
import urllib.request
from pathlib import Path

from playwright.async_api import async_playwright

HERE = Path(__file__).resolve().parent
SERVER = HERE / "server" / "stream_server.py"
PORT = 8080


def sh(*cmd: str) -> str:
    r = subprocess.run(cmd, capture_output=True, text=True)
    return (r.stdout or r.stderr).strip()


def netem(rtt_ms: int) -> None:
    subprocess.run(["tc", "qdisc", "del", "dev", "lo", "root"], capture_output=True)
    if rtt_ms > 0:  # loopback carries both directions, so each direction gets half the RTT
        subprocess.run(["tc", "qdisc", "add", "dev", "lo", "root", "netem", "delay", f"{rtt_ms / 2:g}ms"], check=True)


class Server:
    """A stream server subprocess whose output is drained continuously into a log (never blocks)."""

    def __init__(self, token: str, log_path: Path, *extra: str) -> None:
        self.lines: list[str] = []
        self.log = open(log_path, "a", encoding="utf-8")
        self.p = subprocess.Popen([sys.executable, "-u", str(SERVER), "--bind", "127.0.0.1", "--port", str(PORT), "--token", token, *extra],
                                  stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True, bufsize=1)
        threading.Thread(target=self._drain, daemon=True).start()

    def _drain(self) -> None:
        for line in self.p.stdout:
            self.lines.append(line.rstrip())
            self.log.write(line)
            self.log.flush()

    def marker(self, name: str) -> dict | None:
        for line in reversed(self.lines):
            if line.startswith(name + "="):
                return json.loads(line.split("=", 1)[1])
        return None

    def tail(self, n: int = 60) -> str:
        return "\n".join(self.lines[-n:])


def start_server(token: str, log_path: Path, *extra: str) -> tuple[Server, dict]:
    s = Server(token, log_path, *extra)
    deadline = time.time() + 120
    while time.time() < deadline:
        ready = s.marker("HELQIX_STREAM_READY")
        if ready:
            return s, ready
        if s.p.poll() is not None:
            break
        time.sleep(0.2)
    s.p.kill()
    raise RuntimeError("stream server did not become ready:\n" + s.tail())


def wait_end(s: Server, timeout_s: float) -> tuple[dict | None, float]:
    t0 = time.time()
    while time.time() - t0 < timeout_s and s.p.poll() is None:
        time.sleep(0.25)
    if s.p.poll() is None:
        s.p.kill()
    time.sleep(0.3)
    return s.marker("HELQIX_STREAM_END"), round(time.time() - t0, 1)


def stats(token: str) -> dict:
    with urllib.request.urlopen(f"http://127.0.0.1:{PORT}/stats?token={token}", timeout=10) as r:
        return json.loads(r.read())


async def view(token: str, n: int, relay: bool) -> tuple[dict, str]:
    async with async_playwright() as pw:
        try:
            browser = await pw.chromium.launch(channel="chrome", headless=True, args=["--autoplay-policy=no-user-gesture-required"])
        except Exception:  # Google Chrome not installed: open-source Chromium (may lack H.264)
            browser = await pw.chromium.launch(headless=True, args=["--autoplay-policy=no-user-gesture-required"])
        page = await browser.new_page(viewport={"width": 1280, "height": 720})
        await page.goto(f"http://127.0.0.1:{PORT}/client.html?token={token}&n={n}{'&relay=1' if relay else ''}")
        await page.wait_for_function("window.__results !== undefined", timeout=240000)
        result = await page.evaluate("window.__results")
        version = browser.version
        await browser.close()
        return result, version


def main() -> int:
    a = argparse.ArgumentParser()
    a.add_argument("--out", default="out")
    a.add_argument("--n", type=int, default=40)
    a.add_argument("--encoder", default="auto")
    a.add_argument("--no-turn", action="store_true")
    args = a.parse_args()
    out = Path(args.out)
    out.mkdir(parents=True, exist_ok=True)
    token = secrets.token_urlsafe(16)
    problems: list[str] = []

    turn = None
    turn_proc = None
    if not args.no_turn and shutil.which("turnserver"):
        pw_turn = secrets.token_hex(8)  # per-run, never stored
        turn_proc = subprocess.Popen(["turnserver", "--listening-ip=127.0.0.1", "--relay-ip=127.0.0.1", "--listening-port=3478", f"--user=helqix:{pw_turn}",
                                      "--realm=helqix", "--lt-cred-mech", "--no-tls", "--no-dtls", "--allow-loopback-peers", "--no-cli", "--fingerprint",
                                      "--min-port=49160", "--max-port=49200", "--log-file=stdout"], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        time.sleep(1.0)
        turn = f"127.0.0.1:3478:helqix:{pw_turn}"

    server, ready = start_server(token, out / "server.log", "--encoder", args.encoder, "--idle-timeout", "60", "--max-session", "1800", *(["--turn", turn] if turn else []))
    cases = [("RTT 0 ms", 0, False), ("RTT 40 ms", 40, False), ("RTT 80 ms", 80, False)] + ([("RTT 0 ms via TURN relay", 0, True)] if turn else [])
    results = []
    chrome = None
    for name, rtt, relay in cases:
        netem(rtt)
        try:
            res, chrome = asyncio.run(view(token, args.n, relay))
        except Exception as e:  # noqa: BLE001
            res = {"error": f"{type(e).__name__}: {e}"[:500]}
        finally:
            netem(0)
        time.sleep(2.0)  # let the server record the closed session
        try:
            s = stats(token)
            sess = s["sessions"][-1] if s["sessions"] else {}
        except OSError as e:
            sess = {"error": f"stats unavailable: {e}"}
            problems.append(f"{name}: server not answering (exit code {server.p.poll()}); see server.log")
        results.append({"case": name, "rtt_ms": rtt, "relay": relay, "viewer": res, "server": sess})
        if res.get("error") or not res.get("measured"):
            problems.append(f"{name}: no latency samples ({res.get('error') or res})")
        if server.p.poll() is not None:
            problems.append(f"stream server exited during {name} with code {server.p.returncode}")
            break
    if ready["encoder"] != (args.encoder if args.encoder != "auto" else ready["encoder"]):
        problems.append(f"encoder {ready['encoder']} differs from the request {args.encoder}")

    # Guard 1: no input -> idle shutdown (idle timeout 60 s, longer than a viewer's 30 s start-up wait).
    end_idle, idle_after = wait_end(server, 90)
    if not end_idle or end_idle.get("reason") != "idle" or not end_idle.get("temp_removed"):
        problems.append(f"idle shutdown did not happen as expected: {end_idle}")
    # Guard 2: hard cap of 8 s with a long idle timeout.
    capped, _ = start_server(token, out / "server-hardcap.log", "--encoder", "x264", "--idle-timeout", "600", "--max-session", "8")
    end_cap, cap_after = wait_end(capped, 60)
    if not end_cap or end_cap.get("reason") != "hard-cap":
        problems.append(f"hard-cap shutdown did not happen as expected: {end_cap}")
    if turn_proc:
        turn_proc.terminate()

    env = {
        "encoder": ready["encoder"], "encoder_reason": ready["encoder_reason"], "scene_renderer": ready["renderer"], "viewer_browser": chrome,
        "gstreamer": sh("gst-launch-1.0", "--version").splitlines()[0] if shutil.which("gst-launch-1.0") else None,
        "x264": next((l.strip() for l in sh("gst-inspect-1.0", "x264enc").splitlines() if "Version" in l), None),
        "cpu": platform.processor() or platform.machine(), "cpu_count": os.cpu_count(), "kernel": platform.release(),
        "gpu": sh("nvidia-smi", "--query-gpu=name,driver_version", "--format=csv,noheader") if shutil.which("nvidia-smi") else "none",
        "turn": "coturn " + sh("turnserver", "--version") if turn else "not used",
    }
    doc = {"schema": "helqix.k7-spike.v1", "label": "MEASURED" if not problems else "MEASURED (with failures)", "environment": env,
           "resolution": f"{ready.get('width', 1280)}x{ready.get('height', 720)}", "cases": results,
           "guards": {"idle": {"end": end_idle, "seconds_after_last_viewer": idle_after, "idle_timeout_s": 60}, "hard_cap": {"end": end_cap, "seconds": cap_after, "max_session_s": 8}},
           "problems": problems,
           "note": "Software encode (x264) is the CPU fallback, not a GPU result. Go/no-go needs a GPU run (file 38 section 7)."}
    (out / "results.json").write_text(json.dumps(doc, indent=2) + "\n")
    rows = ["| Case | p50 ms | p95 ms | min / max ms | samples | bitrate kbps | fps | dropped | decode ms/f | encode ms p50 / p95 | CPU % |", "|" + "---|" * 11]
    for r in results:
        v, s = r["viewer"], r["server"]
        lm, vid, el = v.get("latency_ms", {}), v.get("video") or {}, s.get("encode_latency_ms", {})
        rows.append(f"| {r['case']} | {lm.get('p50')} | {lm.get('p95')} | {lm.get('min')} / {lm.get('max')} | {v.get('measured')}/{v.get('samples')} | {vid.get('bitrate_kbps')} | {vid.get('fps')} | "
                    f"{vid.get('frames_dropped')} | {vid.get('decode_ms_per_frame')} | {el.get('p50')} / {el.get('p95')} | {s.get('cpu_percent_of_all_cores_mean')} |")
    md = [f"# K7 spike results ({doc['label']})", "", f"Encoder: **{env['encoder']}** ({env['encoder_reason']}). Scene renderer: {env['scene_renderer']}. Viewer: {env['viewer_browser']}. GPU: {env['gpu']}.", "",
          *rows, "", f"Idle shutdown: {end_idle} after {idle_after} s. Hard cap: {end_cap} after {cap_after} s.", "", doc["note"]]
    (out / "results.md").write_text("\n".join(md) + "\n")
    print("\n".join(md))
    for p in problems:
        print("PROBLEM:", p)
    return 1 if problems else 0


if __name__ == "__main__":
    raise SystemExit(main())
