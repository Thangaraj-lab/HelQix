"""MQTT-style bus: routing, ordering, QoS 1 retry, timeouts, faults."""

import time

import pytest

from helqix_io import BackpressureError, Broker, Client, DisconnectedError, IOTimeoutError, topic_matches


def test_ten_thousand_messages_round_trip_exactly_and_in_order():
    broker = Broker()
    subscriber = Client("sub", broker, capacity_messages=10001)
    subscriber.subscribe("plant/#")
    publisher = Client("pub", broker)
    payloads = [f"reading-{index:05d}".encode() + bytes([index % 251]) for index in range(10000)]
    started = time.perf_counter()
    ids = [publisher.publish("plant/data", payload, qos=0, timeout_s=5.0) for payload in payloads]
    messages = [subscriber.receive(timeout_s=5.0) for _ in range(10000)]
    elapsed = time.perf_counter() - started
    assert [message.payload for message in messages] == payloads
    assert [message.message_id for message in messages] == ids
    assert elapsed < 2.0, f"10000 messages took {elapsed:.2f} s"


def test_topic_wildcards():
    assert topic_matches("a/b/c", "a/b/c")
    assert topic_matches("a/+/c", "a/b/c")
    assert not topic_matches("a/+/c", "a/c")
    assert topic_matches("a/#", "a/b/c")
    assert topic_matches("a/#", "a")
    assert topic_matches("#", "a/b")
    assert not topic_matches("a/b", "a/b/c")
    assert not topic_matches("a/+", "a/b/c")


def test_only_matching_subscribers_receive():
    broker = Broker()
    exact = Client("exact", broker)
    exact.subscribe("home/kitchen/temp")
    wildcard = Client("wild", broker)
    wildcard.subscribe("home/+/temp")
    everything = Client("all", broker)
    everything.subscribe("#")
    publisher = Client("pub", broker)
    publisher.publish("home/kitchen/temp", b"21.5")
    publisher.publish("garden/soil", b"wet")
    assert exact.receive(0.5).payload == b"21.5"
    assert wildcard.receive(0.5).payload == b"21.5"
    assert everything.receive(0.5).topic == "home/kitchen/temp"
    assert everything.receive(0.5).topic == "garden/soil"
    with pytest.raises(IOTimeoutError):
        everything.receive(0.02)


def test_qos1_retry_after_disconnect_then_ack_stops_it():
    broker = Broker()
    subscriber = Client("sub", broker)
    subscriber.subscribe("plant/#")
    publisher = Client("pub", broker)
    subscriber.disconnect()
    publisher.publish("plant/a", b"reading", qos=1)
    assert publisher.retry_unacked() == 1
    subscriber.reconnect()
    assert publisher.retry_unacked() == 1
    message = subscriber.receive(0.5)
    assert message.payload == b"reading"
    subscriber.ack(message.message_id)
    assert publisher.retry_unacked() == 0
    with pytest.raises(IOTimeoutError):
        subscriber.receive(0.02)


def test_qos0_is_never_retried():
    broker = Broker()
    subscriber = Client("sub", broker)
    subscriber.subscribe("plant/#")
    publisher = Client("pub", broker)
    publisher.publish("plant/a", b"reading", qos=0)
    assert publisher.retry_unacked() == 0


def test_receive_timeout_is_bounded():
    broker = Broker()
    subscriber = Client("sub", broker)
    subscriber.subscribe("a")
    started = time.perf_counter()
    with pytest.raises(IOTimeoutError):
        subscriber.receive(timeout_s=0.05)
    elapsed = time.perf_counter() - started
    assert 0.03 <= elapsed <= 1.0


def test_back_pressure_refuses_the_whole_publish():
    broker = Broker()
    subscriber = Client("sub", broker, capacity_messages=2)
    subscriber.subscribe("a/#")
    publisher = Client("pub", broker)
    publisher.publish("a/1", b"one")
    publisher.publish("a/2", b"two")
    started = time.perf_counter()
    with pytest.raises(BackpressureError):
        publisher.publish("a/3", b"three", timeout_s=0.05)
    assert time.perf_counter() - started >= 0.03
    assert subscriber.receive(0.5).payload == b"one"
    assert subscriber.receive(0.5).payload == b"two"
    with pytest.raises(IOTimeoutError):
        subscriber.receive(0.02)
    publisher.publish("a/3", b"three", timeout_s=0.5)
    assert subscriber.receive(0.5).payload == b"three"


def test_corruption_is_detected_never_delivered():
    broker = Broker()
    subscriber = Client("sub", broker)
    subscriber.subscribe("a")
    subscriber.configure_faults(seed=2, corrupt=1.0)
    publisher = Client("pub", broker)
    for index in range(10):
        publisher.publish("a", f"m{index}".encode())
    with pytest.raises(IOTimeoutError):
        subscriber.receive(0.02)
    assert subscriber.stats.received == 0
    assert subscriber.stats.corrupted == 10


def test_disconnect_drops_delivery_and_reconnect_restores_it():
    broker = Broker()
    subscriber = Client("sub", broker)
    subscriber.subscribe("a")
    publisher = Client("pub", broker)
    subscriber.disconnect()
    publisher.publish("a", b"while-away")
    with pytest.raises(DisconnectedError):
        subscriber.receive(0.05)
    subscriber.reconnect()
    with pytest.raises(IOTimeoutError):
        subscriber.receive(0.02)
    publisher.publish("a", b"back")
    assert subscriber.receive(0.5).payload == b"back"
