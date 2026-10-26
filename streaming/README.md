# K7 cloud-render spike (`streaming/`)

A proof that the three.js scene can render on a server and stream to a browser or phone with interaction, with measured input-to-photon latency. This is a spike, not product code: the scene is an Example stand-in for the Studio canvas (UI track). The only hook into the app is this folder.

```
Xvfb display -> Chromium (Playwright, kiosk) showing web/scene.html
  -> encoder process: gst-launch ximagesrc -> x264enc (software) | nvh264enc (NVENC) -> RTP -> local UDP
  -> GStreamer webrtcbin (server/stream_server.py) -> WebRTC -> web/client.html (any browser, phone)
  viewer input -> data channel -> Playwright mouse / wheel / keys on the scene
```

- **GPL boundary:** x264 runs only inside its own `gst-launch-1.0` process; the server receives RTP over local UDP.
- **Encoder:** `--encoder auto` uses NVENC only if a real test encode succeeds, otherwise x264. The choice and the reason are printed and recorded.
- **Latency:** the viewer sends `m` and timestamps it. The scene flips a 64 px marker (red/green) in its next WebGL frame. The viewer finds the new colour in a decoded video frame (`requestVideoFrameCallback`). Both times are on the viewer's clock.
- **Session guards:** idle timeout and hard cap (`server/guard.py`). On either, the encoder, peer, browser and display stop, temp files are removed and the process exits. A paid GPU cannot be left running by a forgotten tab.
- **Access:** every endpoint needs the per-run token printed at start-up. One viewer at a time.
- **TURN:** `--turn host:port:user:password` (coturn) for phones behind NAT.

## Run (CPU, any Linux with Docker)

```bash
docker build -t helqix-stream streaming/
docker run --rm --cap-add NET_ADMIN --shm-size 2g -v "$PWD/out:/out" helqix-stream python3 /app/measure.py --out /out --encoder x264
```

`out/results.md` and `out/results.json` hold latency (p50, p95, min, max) at RTT 0, 40 and 80 ms (tc netem) and through a TURN relay, with bitrate, fps, dropped frames, decode and encode time, CPU, and the idle and hard-cap checks.

## Run on a GPU machine (founder's spike session)

Needs native Linux, an NVIDIA driver, Docker and the NVIDIA Container Toolkit (see `docs/K7_SPIKE_PLAN.md`). NVENC in GStreamer needs the `nvcodec` plugin, which Ubuntu's `gstreamer1.0-plugins-bad` includes and which loads the driver's NVENC library at run time.

```bash
docker run --rm --gpus all -e NVIDIA_DRIVER_CAPABILITIES=all --cap-add NET_ADMIN --shm-size 2g -v "$PWD/out:/out" helqix-stream python3 /app/measure.py --out /out --encoder nvh264
```

Phone test: start the server with a public port and TURN, then open `http://<host>:8080/client.html?token=<token>` on the phone and press "Measure latency":

```bash
docker run --rm --gpus all -e NVIDIA_DRIVER_CAPABILITIES=all -p 8080:8080 helqix-stream python3 /app/server/stream_server.py --encoder nvh264 --turn <public-ip>:3478:<user>:<password>
```

## Licences

GStreamer (LGPL, loaded dynamically), x264 (GPL, separate process only), coturn (BSD-3), aiohttp (Apache-2.0 and MIT), Playwright (Apache-2.0), psutil (BSD-3), three.js 0.170.0 (MIT). The automated test **viewer** is Google Chrome, installed by Playwright (proprietary, test-only, never shipped), because open-source Chromium builds lack H.264. Real users use their own browser.
