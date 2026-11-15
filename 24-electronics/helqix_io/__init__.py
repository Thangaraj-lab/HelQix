"""helqix_io — in-process virtual serial and message-bus ports."""

from .bus import Broker, Client, Message, topic_matches
from .errors import BackpressureError, DisconnectedError, IOTimeoutError
from .faults import FaultProfile, PortStats, decode_frame, encode_frame
from .serial_port import SerialPort, serial_pair

PACKAGE_NAME = "helqix_io"
__version__ = "0.1.0"

__all__ = [
    "BackpressureError",
    "Broker",
    "Client",
    "DisconnectedError",
    "FaultProfile",
    "IOTimeoutError",
    "Message",
    "PortStats",
    "SerialPort",
    "decode_frame",
    "encode_frame",
    "serial_pair",
    "topic_matches",
]
