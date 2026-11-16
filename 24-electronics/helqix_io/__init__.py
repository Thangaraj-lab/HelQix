"""helqix_io — in-process virtual serial and message-bus ports."""

from .bundle import BUNDLE_FILES, VERSION as BUNDLE_VERSION, BundleError, build_bundle
from .bus import Broker, Client, Message, topic_matches
from .errors import BackpressureError, DisconnectedError, IOTimeoutError
from .faults import FaultProfile, PortStats, decode_frame, encode_frame
from .serial_port import SerialPort, serial_pair

PACKAGE_NAME = "helqix_io"
__version__ = "0.1.0"

__all__ = [
    "BackpressureError",
    "Broker",
    "BUNDLE_FILES",
    "BUNDLE_VERSION",
    "BundleError",
    "Client",
    "DisconnectedError",
    "FaultProfile",
    "IOTimeoutError",
    "Message",
    "PortStats",
    "SerialPort",
    "build_bundle",
    "decode_frame",
    "encode_frame",
    "serial_pair",
    "topic_matches",
]
