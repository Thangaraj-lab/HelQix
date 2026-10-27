// K7 spike viewer. Plays the WebRTC stream, forwards mouse / wheel / touch / keys over the "input"
// data channel, and measures input-to-photon latency:
//   send "m" at t0 -> the scene flips its marker (red <-> green) in its next WebGL frame -> capture,
//   encode, network, decode -> this page finds the new colour in a decoded video frame at t1.
// latency = t1 - t0 on this page's clock (one clock, no sync needed).
// Query: ?token=...&n=40 runs n samples automatically and stores the result in window.__results;
// &relay=1 forces TURN relay. Without n, press "Measure latency" (phone test).
const q = new URLSearchParams(location.search);
const token = q.get("token") || "";
const video = document.getElementById("v");
const statusEl = document.getElementById("status");
const status = (s) => { statusEl.textContent = s; };
let channel = null;

async function start() {
  const cfg = await (await fetch(`/config?token=${encodeURIComponent(token)}`)).json();
  const pc = new RTCPeerConnection({ iceServers: cfg.iceServers, iceTransportPolicy: q.get("relay") === "1" ? "relay" : "all" });
  window.__pc = pc;
  const ws = new WebSocket(`${location.protocol === "https:" ? "wss" : "ws"}://${location.host}/signal?token=${encodeURIComponent(token)}`);
  pc.ontrack = (e) => { video.srcObject = e.streams[0] || new MediaStream([e.track]); };
  pc.ondatachannel = (e) => { channel = e.channel; channel.onopen = () => status("connected"); };
  pc.onicecandidate = (e) => { if (e.candidate) ws.send(JSON.stringify({ type: "ice", candidate: e.candidate.candidate, sdpMLineIndex: e.candidate.sdpMLineIndex })); };
  pc.onconnectionstatechange = () => status(pc.connectionState);
  ws.onmessage = async (m) => {
    const msg = JSON.parse(m.data);
    if (msg.type === "offer") {
      await pc.setRemoteDescription({ type: "offer", sdp: msg.sdp });
      const answer = await pc.createAnswer();
      await pc.setLocalDescription(answer);
      ws.send(JSON.stringify({ type: "answer", sdp: answer.sdp }));
    } else if (msg.type === "ice") {
      await pc.addIceCandidate({ candidate: msg.candidate, sdpMLineIndex: msg.sdpMLineIndex }).catch(() => {});
    }
  };
  ws.onclose = () => status("closed");
}

const send = (ev) => { if (channel && channel.readyState === "open") channel.send(JSON.stringify(ev)); };
const norm = (e) => { const r = video.getBoundingClientRect(); return { x: (e.clientX - r.left) / r.width, y: (e.clientY - r.top) / r.height }; };
video.addEventListener("pointermove", (e) => send({ t: "move", ...norm(e) }));
video.addEventListener("pointerdown", (e) => { send({ t: "move", ...norm(e) }); send({ t: "down", button: e.button }); });
video.addEventListener("pointerup", (e) => send({ t: "up", button: e.button }));
video.addEventListener("wheel", (e) => { e.preventDefault(); send({ t: "wheel", dy: e.deltaY }); }, { passive: false });
video.addEventListener("contextmenu", (e) => e.preventDefault());
addEventListener("keydown", (e) => send({ t: "key", key: e.key === " " ? "Space" : e.key }));

// Marker detection on decoded frames.
const probe = document.createElement("canvas");
probe.width = probe.height = 8;
const pctx = probe.getContext("2d", { willReadFrequently: true });
function markerColour() {
  const sx = video.videoWidth / 1280, sy = video.videoHeight / 720;  // marker: 64 px square at (16, 16) in a 1280x720 frame
  pctx.drawImage(video, (16 + 16) * sx, (16 + 16) * sy, 32 * sx, 32 * sy, 0, 0, 8, 8);
  const d = pctx.getImageData(0, 0, 8, 8).data;
  let r = 0, g = 0, b = 0;
  for (let i = 0; i < d.length; i += 4) { r += d[i]; g += d[i + 1]; b += d[i + 2]; }
  const n = d.length / 4; r /= n; g /= n; b /= n;
  window.__lastRgb = [Math.round(r), Math.round(g), Math.round(b)];
  if (r > 150 && g < 110 && b < 110) return "red";
  if (g > 150 && r < 110 && b < 110) return "green";
  return "none";
}

function waitFor(colour, timeoutMs) {
  return new Promise((resolve) => {
    const deadline = performance.now() + timeoutMs;
    const step = (now) => {
      if (markerColour() === colour) return resolve(now);
      if (performance.now() > deadline) return resolve(null);
      video.requestVideoFrameCallback(step);
    };
    video.requestVideoFrameCallback(step);
  });
}

async function inboundStats() {
  const stats = await window.__pc.getStats();
  for (const s of stats.values()) if (s.type === "inbound-rtp" && s.kind === "video") return s;
  return null;
}

async function measure(n) {
  status("measuring");
  const lat = [], timeouts = [];
  let sent = 0;
  let expected = "red";
  const s0 = await inboundStats(), w0 = performance.now();
  for (let i = 0; i < n; i++) {
    await new Promise((r) => setTimeout(r, 150 + Math.random() * 100));
    const t0 = performance.now();
    send({ t: "key", key: "m" });
    sent++;
    const t1 = await waitFor(expected, 3000);
    if (t1 === null) timeouts.push(i); else lat.push(t1 - t0);
    expected = expected === "red" ? "green" : "red";
    if (t1 === null) expected = markerColour() === "red" ? "green" : "red"; // resynchronise after a miss
  }
  const s1 = await inboundStats(), w1 = performance.now();
  lat.sort((a, b) => a - b);
  const pct = (p) => (lat.length ? +lat[Math.min(lat.length - 1, Math.floor(lat.length * p))].toFixed(1) : null);
  const secs = (w1 - w0) / 1000;
  const result = {
    samples: n, measured: lat.length, timeouts: timeouts.length,
    diag: { sent, channel: channel ? channel.readyState : null, last_rgb: window.__lastRgb || null, video_size: [video.videoWidth, video.videoHeight] },
    latency_ms: { min: lat.length ? +lat[0].toFixed(1) : null, p50: pct(0.5), p95: pct(0.95), max: lat.length ? +lat[lat.length - 1].toFixed(1) : null },
    video: s1 && s0 ? {
      width: s1.frameWidth, height: s1.frameHeight, fps: s1.framesPerSecond,
      bitrate_kbps: +(((s1.bytesReceived - s0.bytesReceived) * 8) / secs / 1000).toFixed(1),
      frames_decoded: s1.framesDecoded - s0.framesDecoded, frames_dropped: (s1.framesDropped || 0) - (s0.framesDropped || 0),
      decode_ms_per_frame: s1.framesDecoded > s0.framesDecoded ? +((s1.totalDecodeTime - s0.totalDecodeTime) * 1000 / (s1.framesDecoded - s0.framesDecoded)).toFixed(2) : null,
      jitter_buffer_ms: s1.jitterBufferEmittedCount ? +((s1.jitterBufferDelay / s1.jitterBufferEmittedCount) * 1000).toFixed(1) : null,
      codec: [...(await window.__pc.getStats()).values()].find((s) => s.type === "codec" && s.id === s1.codecId)?.mimeType || null,
    } : null,
  };
  status(`p50 ${result.latency_ms.p50} ms, p95 ${result.latency_ms.p95} ms (${lat.length}/${n})`);
  return result;
}

document.getElementById("measure").onclick = async () => { window.__results = await measure(30); };

start().then(async () => {
  const n = parseInt(q.get("n") || "0", 10);
  if (!n) return;
  // Wait for the channel and a few decoded frames before measuring.
  for (let i = 0; i < 300 && !(channel && channel.readyState === "open" && video.readyState >= 2 && video.videoWidth); i++) await new Promise((r) => setTimeout(r, 100));
  await new Promise((r) => setTimeout(r, 1500));
  window.__results = channel && channel.readyState === "open" && video.videoWidth ? await measure(n) : { error: "stream did not start", state: window.__pc.connectionState, video: video.videoWidth };
});
