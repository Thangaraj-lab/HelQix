"""K7 render-and-stream server (spike). One process serves one viewer at a time:

  Xvfb display  ->  Chromium (Playwright, headful, kiosk) showing web/scene.html
  -> separate encoder process (gst-launch: ximagesrc -> x264enc | nvh264enc -> RTP -> local UDP)
  -> GStreamer webrtcbin (this process) -> WebRTC to the viewer (web/client.html)
  viewer input (data channel) -> Playwright mouse/wheel/keyboard on the scene page.

Signalling is a WebSocket at /signal?token=...; /stats?token=... reports the encoder, encode latency
(GStreamer latency tracer), CPU and GPU samples and the guard state. The session ends on the idle
timeout or the hard cap: encoder, peer, browser and display are stopped and the process exits.
Usage: python3 stream_server.py [--port 8080] [--encoder auto|x264|nvh264] [--width 1280 --height 720 --fps 30]
       [--kbps 4000] [--idle-timeout 120] [--max-session 900] [--turn host:port:user:pass] [--token T]
Prints one line `HELQIX_STREAM_READY=<json>` when ready and `HELQIX_STREAM_END=<json>` on exit.
"""
from __future__ import annotations

import argparse
import asyncio
import json
import os
import secrets
import shutil
import signal
import subprocess
import sys
import tempfile
import threading
import time
from pathlib import Path

import gi

gi.require_version("Gst", "1.0")
gi.require_version("GstWebRTC", "1.0")
gi.require_version("GstSdp", "1.0")
from gi.repository import GLib, Gst, GstSdp, GstWebRTC  # noqa: E402

import psutil  # noqa: E402
from aiohttp import WSMsgType, web  # noqa: E402
from playwright.async_api import async_playwright  # noqa: E402

sys.path.insert(0, str(Path(__file__).resolve().parent))
from encoder import RTP_CAPS, TRACER_ENV, choose, encode_latency_ms, pipeline  # noqa: E402
from guard import SessionGuard  # noqa: E402

WEB = Path(__file__).resolve().parents[1] / "web"
ALLOWED_KEYS = {*"abcdefghijklmnopqrstuvwxyz0123456789", "ArrowUp", "ArrowDown", "ArrowLeft", "ArrowRight", "Escape", "Enter", "Space"}
RTP_PORT = 5004


def log(msg: str) -> None:
    print(f"[stream {time.strftime('%H:%M:%S')}] {msg}", flush=True)


class Sampler:
    """CPU (this process and children) and, if nvidia-smi exists, GPU/encoder utilisation."""

    def __init__(self) -> None:
        self.cpu, self.gpu, self.nvenc = [], [], []
        self._procs: dict[int, psutil.Process] = {}

    def sample(self) -> None:
        me = psutil.Process(os.getpid())
        total = 0.0
        for p in [me, *me.children(recursive=True)]:
            proc = self._procs.setdefault(p.pid, p)
            try:
                total += proc.cpu_percent(None)
            except psutil.Error:
                pass
        self.cpu.append(total / (psutil.cpu_count() or 1))
        if shutil.which("nvidia-smi"):
            r = subprocess.run(["nvidia-smi", "--query-gpu=utilization.gpu,utilization.encoder", "--format=csv,noheader,nounits"], capture_output=True, text=True)
            try:
                g, e = (float(x) for x in r.stdout.strip().splitlines()[0].split(","))
                self.gpu.append(g)
                self.nvenc.append(e)
            except (ValueError, IndexError):
                pass

    def summary(self) -> dict:
        mean = lambda v: round(sum(v) / len(v), 1) if v else None  # noqa: E731
        return {"cpu_percent_of_all_cores_mean": mean(self.cpu[1:]), "gpu_util_mean": mean(self.gpu), "nvenc_util_mean": mean(self.nvenc), "samples": len(self.cpu)}


class Session:
    """One viewer: encoder process + webrtcbin pipeline + signalling socket."""

    def __init__(self, server: "StreamServer", ws: web.WebSocketResponse) -> None:
        self.server, self.ws, self.loop = server, ws, asyncio.get_running_loop()
        self.offer_sent = False
        self.channel = None
        a = server.args
        self.enc_log = Path(server.tmp) / f"encoder-{time.time_ns()}.log"
        env = {**os.environ, **TRACER_ENV, "DISPLAY": a.display}
        self.encoder = subprocess.Popen(["gst-launch-1.0", "-q", *pipeline(server.encoder, a.display, a.width, a.height, a.fps, a.kbps, RTP_PORT).split()],
                                        env=env, stdout=subprocess.DEVNULL, stderr=open(self.enc_log, "w"))
        self.pipe = Gst.parse_launch(f'udpsrc port={RTP_PORT} caps="{RTP_CAPS}" ! queue ! webrtcbin name=webrtc bundle-policy=max-bundle latency=0')
        self.webrtc = self.pipe.get_by_name("webrtc")
        if server.turn:
            self.webrtc.set_property("turn-server", server.turn)
        self.webrtc.connect("on-negotiation-needed", self.on_negotiation_needed)
        self.webrtc.connect("on-ice-candidate", self.on_ice_candidate)
        self.pipe.set_state(Gst.State.PLAYING)
        self.sampler = Sampler()

    def send(self, obj: dict) -> None:
        self.loop.call_soon_threadsafe(lambda: asyncio.ensure_future(self.ws.send_str(json.dumps(obj))))

    def on_negotiation_needed(self, element) -> None:
        if self.offer_sent:
            return
        self.offer_sent = True
        self.channel = element.emit("create-data-channel", "input", None)
        self.channel.connect("on-message-string", lambda _ch, msg: self.loop.call_soon_threadsafe(self.server.on_input, msg))
        promise = Gst.Promise.new_with_change_func(self.on_offer_created, element, None)
        element.emit("create-offer", None, promise)

    def on_offer_created(self, promise, element, _) -> None:
        promise.wait()
        offer = promise.get_reply().get_value("offer")
        element.emit("set-local-description", offer, Gst.Promise.new())
        self.send({"type": "offer", "sdp": offer.sdp.as_text()})

    def on_ice_candidate(self, _element, mline: int, candidate: str) -> None:
        self.send({"type": "ice", "sdpMLineIndex": mline, "candidate": candidate})

    def on_signal(self, msg: dict) -> None:
        if msg.get("type") == "answer":
            _, sdp = GstSdp.SDPMessage.new_from_text(msg["sdp"])
            answer = GstWebRTC.WebRTCSessionDescription.new(GstWebRTC.WebRTCSDPType.ANSWER, sdp)
            self.webrtc.emit("set-remote-description", answer, Gst.Promise.new())
        elif msg.get("type") == "ice" and msg.get("candidate"):
            self.webrtc.emit("add-ice-candidate", int(msg.get("sdpMLineIndex") or 0), msg["candidate"])

    def close(self) -> dict:
        self.pipe.set_state(Gst.State.NULL)
        self.encoder.send_signal(signal.SIGINT)
        try:
            self.encoder.wait(timeout=5)
        except subprocess.TimeoutExpired:
            self.encoder.kill()
        text = self.enc_log.read_text(errors="replace") if self.enc_log.exists() else ""
        return {"encode_latency_ms": encode_latency_ms(text), "encoder_exit": self.encoder.returncode, **self.sampler.summary()}


class StreamServer:
    def __init__(self, args) -> None:
        self.args = args
        self.token = args.token or secrets.token_urlsafe(16)
        self.turn = None
        self.ice_servers = []
        if args.turn:
            host, port, user, pw = args.turn.split(":")
            self.turn = f"turn://{user}:{pw}@{host}:{port}"
            self.ice_servers = [{"urls": f"turn:{host}:{port}?transport=udp", "username": user, "credential": pw}]
        self.guard = SessionGuard(args.idle_timeout, args.max_session)
        self.session: Session | None = None
        self.finished: list[dict] = []
        self.tmp = tempfile.mkdtemp(prefix="helqix-stream-")
        self.encoder, self.encoder_reason = choose(args.encoder)
        self.end_reason: str | None = None

    # --- input from the viewer (asyncio thread) ---
    def on_input(self, text: str) -> None:
        try:
            ev = json.loads(text)
        except ValueError:
            return
        self.guard.touch()
        asyncio.ensure_future(self.apply_input(ev))

    async def apply_input(self, ev: dict) -> None:
        a, page = self.args, self.page
        clamp = lambda v: min(max(float(v), 0.0), 1.0)  # noqa: E731
        t = ev.get("t")
        if t == "move":
            await page.mouse.move(clamp(ev["x"]) * a.width, clamp(ev["y"]) * a.height)
        elif t == "down":
            await page.mouse.down(button="right" if ev.get("button") == 2 else "left")
        elif t == "up":
            await page.mouse.up(button="right" if ev.get("button") == 2 else "left")
        elif t == "wheel":
            await page.mouse.wheel(0, max(min(float(ev.get("dy", 0)), 1000), -1000))
        elif t == "key" and ev.get("key") in ALLOWED_KEYS:
            await page.keyboard.press(" " if ev["key"] == "Space" else ev["key"])

    # --- HTTP ---
    def authorised(self, request) -> bool:
        return secrets.compare_digest(request.query.get("token", ""), self.token)

    async def signal_handler(self, request):
        if not self.authorised(request):
            return web.Response(status=401, text="token required")
        if self.session is not None:
            return web.Response(status=409, text="a session is already active")
        ws = web.WebSocketResponse(heartbeat=10)
        await ws.prepare(request)
        self.guard.touch()
        self.session = Session(self, ws)
        log(f"viewer connected ({self.encoder})")
        try:
            async for msg in ws:
                if msg.type == WSMsgType.TEXT:
                    self.session.on_signal(json.loads(msg.data))
        finally:
            result = self.session.close()
            self.finished.append(result)
            self.session = None
            log(f"viewer left: {json.dumps(result)}")
        return ws

    async def stats_handler(self, request):
        if not self.authorised(request):
            return web.Response(status=401, text="token required")
        state, remaining = self.guard.state()
        return web.json_response({"encoder": self.encoder, "encoder_reason": self.encoder_reason, "sessions": self.finished,
                                  "active": self.session is not None, "guard": {"state": state, "remaining_s": round(remaining, 1)},
                                  "width": self.args.width, "height": self.args.height, "fps": self.args.fps, "kbps": self.args.kbps})

    async def config_handler(self, request):
        if not self.authorised(request):
            return web.Response(status=401, text="token required")
        return web.json_response({"iceServers": self.ice_servers, "width": self.args.width, "height": self.args.height})

    # --- lifecycle ---
    async def run(self) -> int:
        a = self.args
        self.xvfb = subprocess.Popen(["Xvfb", a.display, "-screen", "0", f"{a.width}x{a.height}x24", "-nolisten", "tcp"], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        for _ in range(50):
            if Path(f"/tmp/.X11-unix/X{a.display.lstrip(':')}").exists():
                break
            await asyncio.sleep(0.1)
        Gst.init(None)
        threading.Thread(target=GLib.MainLoop().run, daemon=True).start()

        app = web.Application()
        app.router.add_get("/signal", self.signal_handler)
        app.router.add_get("/stats", self.stats_handler)
        app.router.add_get("/config", self.config_handler)
        app.router.add_static("/vendor/three/", WEB / "node_modules" / "three")
        app.router.add_static("/", WEB)
        runner = web.AppRunner(app)
        await runner.setup()
        await web.TCPSite(runner, a.bind, a.port).start()

        pw = await async_playwright().start()
        gl_args = ["--ignore-gpu-blocklist", "--enable-unsafe-swiftshader"] if self.encoder == "x264" else ["--ignore-gpu-blocklist", "--use-angle=vulkan", "--enable-features=Vulkan"]
        self.browser = await pw.chromium.launch(headless=False, env={**os.environ, "DISPLAY": a.display},
                                                args=["--kiosk", "--window-position=0,0", f"--window-size={a.width},{a.height}", "--disable-infobars", "--noerrdialogs", *gl_args])
        self.page = await self.browser.new_page(viewport={"width": a.width, "height": a.height})
        await self.page.goto(f"http://127.0.0.1:{a.port}/scene.html")
        await self.page.wait_for_function("window.sceneReady === true", timeout=60000)
        renderer = await self.page.evaluate("window.rendererInfo")
        print("HELQIX_STREAM_READY=" + json.dumps({"port": a.port, "token": self.token, "encoder": self.encoder, "encoder_reason": self.encoder_reason,
                                                    "renderer": renderer, "idle_timeout_s": a.idle_timeout, "max_session_s": a.max_session}), flush=True)

        sampler_tick = 0.0
        while True:
            state, _ = self.guard.state()
            if state != "ok":
                self.end_reason = state
                break
            if self.session and time.monotonic() - sampler_tick >= 1.0:
                self.session.sampler.sample()
                sampler_tick = time.monotonic()
            await asyncio.sleep(0.25)

        log(f"session limit reached: {self.end_reason}; shutting down")
        if self.session:
            await self.session.ws.close()
            await asyncio.sleep(0.5)
        await self.browser.close()
        await pw.stop()
        await runner.cleanup()
        self.xvfb.terminate()
        try:
            self.xvfb.wait(timeout=5)
        except subprocess.TimeoutExpired:
            self.xvfb.kill()
        shutil.rmtree(self.tmp, ignore_errors=True)
        print("HELQIX_STREAM_END=" + json.dumps({"reason": self.end_reason, "sessions": len(self.finished), "temp_removed": not Path(self.tmp).exists()}), flush=True)
        return 0


def main() -> int:
    p = argparse.ArgumentParser()
    p.add_argument("--bind", default="0.0.0.0")
    p.add_argument("--port", type=int, default=8080)
    p.add_argument("--display", default=":99")
    p.add_argument("--encoder", default="auto", choices=["auto", "x264", "nvh264"])
    p.add_argument("--width", type=int, default=1280)
    p.add_argument("--height", type=int, default=720)
    p.add_argument("--fps", type=int, default=30)
    p.add_argument("--kbps", type=int, default=4000)
    p.add_argument("--idle-timeout", type=float, default=120)
    p.add_argument("--max-session", type=float, default=900)
    p.add_argument("--turn", default=None, help="host:port:user:password")
    p.add_argument("--token", default=None)
    return asyncio.run(StreamServer(p.parse_args()).run())


if __name__ == "__main__":
    raise SystemExit(main())
