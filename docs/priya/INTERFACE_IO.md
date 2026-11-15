# HELQIX virtual I/O (helqix_io)

`helqix_io` is a small reusable building block for tests, the sensor demos and
the Studio: a serial-like cable pair and an MQTT-style message bus, both fully
in-process. It uses only the Python standard library: no broker, no sockets,
no threads and no network at run time.

It is **not** wire-compatible with MQTT and is **not** a replacement for a real
transport. It reproduces the semantics a test needs — ordering, framing,
timeouts, back-pressure, acknowledgement and reproducible faults — nothing more.

## Quick start

```python
from helqix_io import serial_pair

left, right = serial_pair()
left.write(b"ping")
assert right.read(timeout_s=1.0) == b"ping"
```

```python
from helqix_io import Broker, Client

broker = Broker()
sensor = Client("sensor", broker)
sensor.subscribe("plant/+/temperature")
gateway = Client("gateway", broker)
gateway.publish("plant/a/temperature", b"21.5")
message = sensor.receive(timeout_s=1.0)
sensor.ack(message.message_id)
```

## Serial pair

`serial_pair(capacity_frames=4096)` returns two `SerialPort` objects wired as
the two ends of one cable. `write(payload, timeout_s=1.0)` sends exactly one
payload; `read(timeout_s=1.0)` returns exactly one payload, in order. Each
frame is `4-byte big-endian length | payload | CRC32`; empty and binary
payloads are legal, and a frame whose CRC does not match is never returned.

- The receiver queue is bounded. A writer whose peer queue is full waits (1 ms
  poll) and raises `BackpressureError` after its deadline; nothing partial is
  ever delivered.
- `read` raises `IOTimeoutError` (a `TimeoutError`) after its deadline.
- `disconnect()` closes both ends of the pair and clears queued frames;
  `reconnect()` starts clean. Traffic while closed raises `DisconnectedError`.
- `configure_faults(seed, drop, corrupt, delay_s, duplicate, reorder)` installs
  faults on the receiving port: frames are dropped, corrupted (one byte flipped,
  so CRC fails), delayed, duplicated or swapped with the next ready frame. The
  same seed always produces the same outcome.
- A duplicate is delivered on the next read, before any newer frame, and is
  not itself subject to drop or corrupt. A reorder read requires a second
  frame that is already ready and swaps the two oldest ready frames.
- `stats` counts `sent, received, dropped, corrupted, timeouts, retries, acks,
  duplicated, reordered`. `received` includes duplicate deliveries, so
  `sent == received - duplicated + dropped + corrupted`.

## Message bus

`Broker` routes by topic. `Client.publish(topic, payload, qos=0|1, timeout_s)`
and `Client.receive(timeout_s)` behave like the serial pair: FIFO per client,
bounded queue, `BackpressureError` when a target queue is full (the whole
publish is refused), `IOTimeoutError` on an empty read.

- Subscriptions use MQTT wildcards: `+` matches one level, `#` matches the
  rest of the tree. Matching was checked against a real broker (below).
- QoS 0 is fire-and-forget. QoS 1 keeps the message pending until
  `ack(message_id)`; `retry_unacked()` resends every unacknowledged message
  under a new message id. Unlike MQTT persistent sessions, a broker does not
  queue messages for a disconnected client: reconnect, then call
  `retry_unacked()` on the publisher.
- The sender's own subscription receives the message, as in MQTT.
- `configure_faults` applies to what this client receives (drop, corrupt,
  delay, duplicate, reorder), seeded per client, with the same semantics as the
  serial pair.

## Measured behaviour

| what | result | label |
|------|--------|-------|
| 10000 serial frames round trip, in order | 0.049 s, every payload identical | MEASURED |
| 10000 bus messages round trip, in order | 0.080 s, unique ids, every payload identical | MEASURED |
| suite (fast) | 35 tests | MEASURED |
| suite (nightly) | 20 seeded property cases, seeds 0..19 (55 tests in total) | MEASURED |
| mutation checks | 18 io and probe mutations, 18 caught | MEASURED |
| wildcard agreement with amqtt 0.12.2 | 49 of 49 filter/topic pairs agree | MEASURED |
| live probe, 7 scenarios x 2 transports, seed 11 | 120 readings: drop 65, corrupt 65, duplicate 120, reorder 119, combined 96 unique + 12 dropped + 12 corrupted + 10 duplicates; serial and bus counters identical | MEASURED |

The interop probe ran once in a throwaway venv (Python 3.13.12, amqtt 0.12.2,
paho-mqtt) against a real broker on 127.0.0.1; it needs network to install and
is not part of the offline suites. Interop covers topic-matching decisions
only; wire-protocol compatibility is UNKNOWN because it is deliberately not
implemented.

`scripts/live_probe.py` sends a seeded sensor trace through both transports
into a consumer for seven scenarios: clean, drop, corrupt, delay, reorder,
duplicate and a combined run. The expected counters are calculated from the
same seed with a separate `random.Random` stream, so a mismatch fails the run.
Repeating seed 11 reproduces the report byte for byte; seed 29 differs.

## Limits

- Single process only; no serialisation guarantees across processes.
- Not MQTT wire compatible, no TLS, no retained messages, no persistent
  sessions, no QoS 2.
- Memory is bounded by the configured queue capacities.
- Timeouts are cooperative and polled at 1 ms, so a timeout can overshoot by
  one poll interval.
