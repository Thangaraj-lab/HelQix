# K7 cloud-render spike: plan (file 38 Prompt B, file 39 K7)

Goal: prove that the three.js scene can render on a GPU machine and stream to a browser or phone with interaction, measure input-to-photon latency at RTT 0, 40 and 80 ms, and give a go/no-go against file 38 section 7 (T2 under 100 ms at RTT 40 ms; the phone plays smoothly).

## What the GPU machine needs

| Need | Why | Minimum |
|---|---|---|
| NVIDIA GPU with NVENC | hardware H.264 encode (`nvh264enc`) | Turing or newer: T4, L4, RTX 20xx/30xx/40xx/50xx |
| **Native Linux** (Ubuntu 22.04 or 24.04) | NVIDIA Container Toolkit, GPU Chromium, `tc netem` | not WSL2: a 2026 report says NVENC works in WSL2 containers but GPU rendering falls back to software (llvmpipe), which would make the spike measure the wrong thing |
| NVIDIA driver, Docker, NVIDIA Container Toolkit | run the stream container with `--gpus all` | Toolkit 1.18.x if 1.19.1 shows the reported encoder crash |
| Root, with `NET_ADMIN` | `tc netem` adds 40 and 80 ms RTT | a full VM or own machine; GPU *container* rentals usually block this |
| Public IP with UDP open (or TURN on TCP 443) | WebRTC to the founder's phone | open UDP 3478 and a media port range, or coturn on 443 |
| 4+ vCPU, 16 GB RAM, 50 GB disk | Chromium + GStreamer + LiveKit + coturn | |
| Time | | about 3 h setup + 3 h measuring; budget 10 machine-hours |

## Options (prices: third-party aggregators, checked 2026-10-26; confirm on the provider page)

| Option | GPU | Fits? | Cost for ~10 h |
|---|---|---|---|
| **A. AWS EC2 `g4dn.xlarge`** (full VM) | T4 16 GB, NVENC | yes: root, public IP, UDP, netem | about $0.53/h on-demand in US East, about $0.31/h spot: **about $5 compute**. Egress at the file 38 assumption (8 Mbps, $0.09/GB): about $0.32/h. **Under $20 total.** The Mumbai (ap-south-1) price and availability are UNKNOWN; check before launch. |
| B. RTX 4050 laptop (file 38 assumes one) booted into **native Ubuntu** (live USB or dual boot) | RTX 4050, NVENC | yes, if it exists; the founder's current PC has an AMD 840M and **no** NVIDIA GPU | $0 |
| C. RunPod / Vast.ai GPU rental | RTX 4090 / L4 | **poor fit**: container rentals usually give no root `NET_ADMIN` for netem and restricted UDP for WebRTC | about $0.14–0.89/h |
| D. Credits (GitHub Student Pack, AWS Activate, Azure for Students, NVIDIA Inception) | | as A | $0 if granted; eligibility UNKNOWN |

**Recommendation: A, in the AWS region nearest the founder** (Mumbai if g4dn is offered there), on-demand, stopped after each session, with an AWS budget alarm set at $25. A US region adds about 200+ ms real RTT from India, which would swamp the 0/40/80 ms test points. B is free and better if the founder has, or can borrow, an NVIDIA laptop.

## What I can build before any GPU (all runs on CPU, verified in Linux CI)

Scope stays inside a new `streaming/` folder plus a `?render=stream` flag, as file 38 requires. No UI restyling.

1. **Container image:** Ubuntu, Playwright Chromium, GStreamer, LiveKit server (Apache-2.0), coturn (BSD-3). Everything pinned; every licence goes into the tool register.
2. **Scene page:** a minimal three.js page loading the K6 Example bracket, plus a **latency marker**: a corner square that changes colour on each input event. This is a stand-in for the Studio canvas, which the UI track owns.
3. **Pipeline:** headless Chromium → frame capture → **encoder chosen at start-up**: `nvh264enc` if NVENC is found, otherwise `x264enc` (software). Then WebRTC through LiveKit, with TURN through coturn.
4. **Client page:** plays the stream and sends mouse, wheel and keys back over a data channel to drive orbit, zoom and select in the headless page.
5. **Measurement harness:** the client stamps each input. The page flips the marker. The client finds the colour change in the decoded video frame (`requestVideoFrameCallback` plus a pixel read), giving **input-to-photon latency** on one clock. Runs at RTT 0, 40 and 80 ms with `tc netem`. Also records bitrate, dropped frames, encode time, CPU and GPU use.
6. **Session guards:** idle auto-shutdown, a hard session time limit and clean-up, with tests.
7. **CI job:** runs everything in software mode and publishes a **software-encode baseline** (labelled software, not GPU). That is the "fallback and its cost" number file 38 asks for. It also proves the whole path works before money is spent on a GPU.

After that, the GPU session only swaps the encoder and turns on GPU Chromium. It's one script to run on the machine.

## What only the founder can do

- Choose A, B or D. For A: create the AWS account, add payment, set the budget alarm, and launch the instance from my one-command script. (I cannot create accounts or enter payment details.)
- Run the phone test on the Android phone named in file 38 (Narzo 70 Turbo) and read the on-screen latency numbers.
- After the spike: the go/no-go decision, and whether K8 (render sessions and metering) proceeds. That needs written approval.

## Honesty rules for the result

Every number is labelled measured (where, which encoder, which RTT) or assumed. Nothing is claimed for NVENC unless the run shows `nvh264enc` actually encoding. Software-mode numbers are never presented as GPU numbers.
