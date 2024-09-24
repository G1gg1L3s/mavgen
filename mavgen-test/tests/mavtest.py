import argparse
import contextlib
import importlib.util
import inspect
import random
import socket
import subprocess
import sys
from collections.abc import Generator
from pathlib import Path
from typing import ClassVar, Optional, Protocol, Union, cast

from pymavlink import mavutil


class MavlinkMessageProtocol(Protocol):
    fieldnames: ClassVar[list[str]]
    ordered_fieldnames: ClassVar[list[str]]
    fieldtypes: ClassVar[list[str]]
    fieldenums_by_name: ClassVar[dict[str, str]]
    orders: ClassVar[list[int]]
    array_lengths: ClassVar[list[int]]

    def to_dict(self) -> dict[str, Union[int, float, str]]:
        raise NotImplementedError()

    def pack(self, force_mavlink1: bool = False) -> bytes:
        raise NotImplementedError()


class EnumEntryProtocol(Protocol):
    name: str


def import_mod(name: str, path: Path):
    spec = importlib.util.spec_from_file_location(name, path)
    assert spec is not None
    assert spec.loader is not None

    module = importlib.util.module_from_spec(spec)
    sys.modules[name] = module
    spec.loader.exec_module(module)

    return module


def set_dialect(path: Path):
    module_name = path.stem
    module = import_mod(module_name, path)

    mavutil.mavlink = module
    mavutil.current_dialect = module_name
    return module


def extract_mavlink_messages(module) -> Generator[type[MavlinkMessageProtocol], None, None]:
    mav_link_message = module.MAVLink_message

    for _, obj in inspect.getmembers(module, inspect.isclass):
        if (
            issubclass(obj, mav_link_message)
            and obj is not mav_link_message
            and obj is not module.MAVLink_bad_data
            and obj is not module.MAVLink_unknown
        ):
            yield cast(type[MavlinkMessageProtocol], obj)


UNSIGNED_RANGES = {
    'uint8_t': (0, 255),
    'uint16_t': (0, 65535),
    'uint32_t': (0, 4294967295),
    'uint64_t': (0, 18446744073709551615),
}

SIGNED_RANGES = {
    'int8_t': (-128, 127),
    'int16_t': (-32768, 32767),
    'int32_t': (-2147483648, 2147483647),
    'int64_t': (-9223372036854775808, 9223372036854775807),
}

FLOAT_RANGES = {
    'float': (-1000, 1000),
    'double': (-2000, 2000),
}


def gen_random_value(field_type: str):
    if field_type in UNSIGNED_RANGES:
        low, high = UNSIGNED_RANGES[field_type]
        return random.randint(low, high)
    elif field_type in SIGNED_RANGES:
        low, high = SIGNED_RANGES[field_type]
        return random.randint(low, high)
    elif field_type in FLOAT_RANGES:
        low, high = FLOAT_RANGES[field_type]
        value = random.randint(low, high)
        return value / 10
    elif field_type == 'char':
        return chr(random.randint(32, 126)).encode()  # Printable ASCII range
    else:
        raise ValueError(f'Unknown field type: {field_type}')


def gen_random_field(dialect_module, field_type: str, enum_type: Optional[str]):
    if enum_type:
        enum: dict[int, EnumEntryProtocol] = dialect_module.enums[enum_type]
        options = list(key for key, option in enum.items() if not option.name.endswith('_END'))
        return random.choice(options)
    else:
        return gen_random_value(field_type)


def generate_random_message(dialect_module, cls: type[MavlinkMessageProtocol]):
    message = {}

    for i, (name, field_type) in enumerate(zip(cls.fieldnames, cls.fieldtypes)):
        enum_type = cls.fieldenums_by_name.get(name)

        position_in_ordered_fieldname = cls.orders[i]
        length = cls.array_lengths[position_in_ordered_fieldname]

        if length > 0:
            value = [gen_random_field(dialect_module, field_type, enum_type) for _ in range(length)]
            if field_type == 'char':
                value = b''.join(value)

        else:
            value = gen_random_field(dialect_module, field_type, enum_type)

        message[name] = value

    return cls(**message)


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser('mavtest')
    parser.add_argument(
        '--mavgen-test',
        type=Path,
        required=True,
        help='Path to mavgen-test binary',
    )
    parser.add_argument(
        '--dialect',
        type=Path,
        required=True,
        help='Path to module with mavlink dialect',
    )
    return parser.parse_args()


def test(module, address: str, process: contextlib.AbstractContextManager):
    connection: mavutil.mavfile = mavutil.mavlink_connection(address)

    with process, contextlib.closing(connection):
        if connection.wait_heartbeat(timeout=5) is None:
            raise TimeoutError('no heartbeat arrived')

        connection.mav.heartbeat_send(
            mavutil.mavlink.MAV_TYPE_ONBOARD_CONTROLLER,
            mavutil.mavlink.MAV_AUTOPILOT_INVALID,
            0,
            0,
            0,
        )

        for message_class in extract_mavlink_messages(module):
            message = generate_random_message(module, message_class)
            connection.mav.send(message)

            print(f'SENT {message_class.__name__}')

            response = connection.recv_match(blocking=True, timeout=5)
            if response is None:
                raise TimeoutError(f'cannot receive {message_class.__name__}')

            packed = message.pack(connection.mav)
            decoded = connection.mav.decode(bytearray(packed))

            message_dict = decoded.to_dict()
            response_dict = response.to_dict()

            assert message_dict == response_dict, f'{message_dict=}, {response_dict=}'


def allocate_tcp_port() -> int:
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as sock:
        sock.bind(('127.0.0.1', 0))
        return sock.getsockname()[1]


@contextlib.contextmanager
def start_server(mavgen_test_bin: Path, dialect: str, address: str):
    args = [mavgen_test_bin, '--dialect', dialect, '--address', address]
    with subprocess.Popen(args) as proc:
        try:
            yield
        finally:
            try:
                code = proc.wait(1)
            except subprocess.TimeoutExpired:
                proc.terminate()
                code = proc.wait(1)

            if code != 0:
                raise subprocess.SubprocessError(
                    f'{mavgen_test_bin} exited with non-zero code: {code}'
                )


def main():
    args = parse_args()
    dialect: Path = args.dialect
    module = set_dialect(dialect)

    port = allocate_tcp_port()

    popen = start_server(args.mavgen_test, dialect.stem.lower(), f'tcpout:127.0.0.1:{port}')
    test(module, f'tcpin:127.0.0.1:{port}', popen)


if __name__ == '__main__':
    main()
