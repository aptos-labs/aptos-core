# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

"""
This is a simple BCS serializer and deserializer. Learn more at https://github.com/diem/bcs
"""

from __future__ import annotations

import io
import typing
import unittest

MAX_U8 = 2**8 - 1
MAX_U16 = 2**16 - 1
MAX_U32 = 2**32 - 1
MAX_U64 = 2**64 - 1
MAX_U128 = 2**128 - 1


class Deserializer:
    _input: io.BytesIO
    _length: int

    def __init__(self, data: bytes):
        self._length = len(data)
        self._input = io.BytesIO(data)

    def remaining(self) -> int:
        return self._length - self._input.tell()

    def bool(self) -> bool:
        value = int.from_bytes(self._read(1), byteorder="little", signed=False)
        if value == 0:
            return False
        elif value == 1:
            return True
        else:
            raise Exception("Unexpected boolean value: ", value)

    def bytes(self) -> bytes:
        return self._read(self.uleb128())

    def fixed_bytes(self, length: int) -> bytes:
        return self._read(length)

    def map(
        self,
        key_decoder: typing.Callable[[Deserializer], typing.Any],
        value_decoder: typing.Callable[[Deserializer], typing.Any],
    ) -> Dict[typing.Any, typing.Any]:
        length = self.uleb128()
        values = {}
        while len(values) < length:
            key = key_decoder(self)
            value = value_decoder(self)
            values[key] = value
        return values

    def sequence(
        self,
        value_decoder: typing.Callable[[Deserializer], typing.Any],
    ) -> List[typing.Any]:
        length = self.uleb128()
        values = []
        while len(values) < length:
            values.append(value_decoder(self))
        return values

    def str(self) -> str:
        return self.bytes().decode()

    def struct(self, struct: typing.Any) -> typing.Any:
        return struct.deserialize(self)

    def u8(self) -> int:
        return self._read_int(1)

    def u16(self) -> int:
        return self._read_int(2)

    def u32(self) -> int:
        return self._read_int(4)

    def u64(self) -> int:
        return self._read_int(8)

    def u128(self) -> int:
        return self._read_int(16)

    def uleb128(self) -> int:
        value = 0
        shift = 0

        while value <= MAX_U32:
            byte = self._read_int(1)
            value |= (byte & 0x7F) << shift
            if byte & 0x80 == 0:
                break
            shift += 7

        if value > MAX_U128:
            raise Exception("Unexpectedly large uleb128 value")

        return value

    def _read(self, length: int) -> bytes:
        value = self._input.read(length)
        if value is None or len(value) < length:
            actual_length = 0 if value is None else len(value)
            error = (
                f"Unexpected end of input. Requested: {length}, found: {actual_length}"
            )
            raise Exception(error)
        return value

    def _read_int(self, length: int) -> int:
        return int.from_bytes(self._read(length), byteorder="little", signed=False)


class Serializer:
    _output: io.BytesIO

    def __init__(self):
        self._output = io.BytesIO()

    def output(self) -> bytes:
        return self._output.getvalue()

    def bool(self, value: bool):
        self._write_int(int(value), 1)

    def bytes(self, value: bytes):
        self.uleb128(len(value))
        self._output.write(value)

    def fixed_bytes(self, value):
        self._output.write(value)

    def map(
        self,
        values: typing.Dict[typing.Any, typing.Any],
        key_encoder: typing.Callable[[Serializer, typing.Any], bytes],
        value_encoder: typing.Callable[[Serializer, typing.Any], bytes],
    ):
        encoded_values = []
        for (key, value) in values.items():
            encoded_values.append(
                (encoder(key, key_encoder), encoder(value, value_encoder))
            )
        encoded_values.sort(key=lambda item: item[0])

        self.uleb128(len(encoded_values))
        for (key, value) in encoded_values:
            self.fixed_bytes(key)
            self.fixed_bytes(value)

    def sequence_serializer(
        value_encoder: typing.Callable[[Serializer, typing.Any], bytes],
    ):
        return lambda self, values: self.sequence(values, value_encoder)

    def sequence(
        self,
        values: typing.List[typing.Any],
        value_encoder: typing.Callable[[Serializer, typing.Any], bytes],
    ):
        self.uleb128(len(values))
        for value in values:
            self.fixed_bytes(encoder(value, value_encoder))

    def str(self, value: str):
        self.bytes(value.encode())

    def struct(self, value: typing.Any):
        value.serialize(self)

    def u8(self, value: int):
        if value > MAX_U8:
            raise Exception(f"Cannot encode {value} into u8")

        self._write_int(value, 1)

    def u16(self, value: int):
        if value > MAX_U16:
            raise Exception(f"Cannot encode {value} into u16")

        self._write_int(value, 2)

    def u32(self, value: int):
        if value > MAX_U32:
            raise Exception(f"Cannot encode {value} into u32")

        self._write_int(value, 4)

    def u64(self, value: int):
        if value > MAX_U64:
            raise Exception(f"Cannot encode {value} into u64")

        self._write_int(value, 8)

    def u128(self, value: int):
        if value > MAX_U128:
            raise Exception(f"Cannot encode {value} into u128")

        self._write_int(value, 16)

    def uleb128(self, value: int):
        if value > MAX_U32:
            raise Exception(f"Cannot encode {value} into uleb128")

        while value >= 0x80:
            # Write 7 (lowest) bits of data and set the 8th bit to 1.
            byte = value & 0x7F
            self.u8(byte | 0x80)
            value >>= 7

        # Write the remaining bits of data and set the highest bit to 0.
        self.u8(value & 0x7F)

    def _write_int(self, value: int, length: int):
        self._output.write(value.to_bytes(length, "little", signed=False))


def encoder(
    value: typing.Any, encoder: typing.Callable[[Serializer, typing.Any], None]
) -> bytes:
    ser = Serializer()
    encoder(ser, value)
    return ser.output()


class Test(unittest.TestCase):
    def test_bool_true(self):
        in_value = True

        ser = Serializer()
        ser.bool(in_value)
        der = Deserializer(ser.output())
        out_value = der.bool()

        self.assertEqual(in_value, out_value)

    def test_bool_false(self):
        in_value = False

        ser = Serializer()
        ser.bool(in_value)
        der = Deserializer(ser.output())
        out_value = der.bool()

        self.assertEqual(in_value, out_value)

    def test_bool_error(self):
        ser = Serializer()
        ser.u8(32)
        der = Deserializer(ser.output())
        with self.assertRaises(Exception):
            der.bool()

    def test_bytes(self):
        in_value = b"1234567890"

        ser = Serializer()
        ser.bytes(in_value)
        der = Deserializer(ser.output())
        out_value = der.bytes()

        self.assertEqual(in_value, out_value)

    def test_map(self):
        in_value = {"a": 12345, "b": 99234, "c": 23829}

        ser = Serializer()
        ser.map(in_value, Serializer.str, Serializer.u32)
        der = Deserializer(ser.output())
        out_value = der.map(Deserializer.str, Deserializer.u32)

        self.assertEqual(in_value, out_value)

    def test_sequence(self):
        in_value = ["a", "abc", "def", "ghi"]

        ser = Serializer()
        ser.sequence(in_value, Serializer.str)
        der = Deserializer(ser.output())
        out_value = der.sequence(Deserializer.str)

        self.assertEqual(in_value, out_value)

    def test_sequence_serializer(self):
        in_value = ["a", "abc", "def", "ghi"]

        ser = Serializer()
        seq_ser = Serializer.sequence_serializer(Serializer.str)
        seq_ser(ser, in_value)
        der = Deserializer(ser.output())
        out_value = der.sequence(Deserializer.str)

        self.assertEqual(in_value, out_value)

    def test_str(self):
        in_value = "1234567890"

        ser = Serializer()
        ser.str(in_value)
        der = Deserializer(ser.output())
        out_value = der.str()

        self.assertEqual(in_value, out_value)

    def test_u8(self):
        in_value = 15

        ser = Serializer()
        ser.u8(in_value)
        der = Deserializer(ser.output())
        out_value = der.u8()

        self.assertEqual(in_value, out_value)

    def test_u16(self):
        in_value = 11115

        ser = Serializer()
        ser.u16(in_value)
        der = Deserializer(ser.output())
        out_value = der.u16()

        self.assertEqual(in_value, out_value)

    def test_u32(self):
        in_value = 1111111115

        ser = Serializer()
        ser.u32(in_value)
        der = Deserializer(ser.output())
        out_value = der.u32()

        self.assertEqual(in_value, out_value)

    def test_u64(self):
        in_value = 1111111111111111115

        ser = Serializer()
        ser.u64(in_value)
        der = Deserializer(ser.output())
        out_value = der.u64()

        self.assertEqual(in_value, out_value)

    def test_u128(self):
        in_value = 1111111111111111111111111111111111115

        ser = Serializer()
        ser.u128(in_value)
        der = Deserializer(ser.output())
        out_value = der.u128()

        self.assertEqual(in_value, out_value)

    def test_uleb128(self):
        in_value = 1111111115

        ser = Serializer()
        ser.uleb128(in_value)
        der = Deserializer(ser.output())
        out_value = der.uleb128()

        self.assertEqual(in_value, out_value)


if __name__ == "__main__":
    unittest.main()
