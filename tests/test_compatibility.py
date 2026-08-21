import array
import base64
import sys

import base64_rs
import pytest

ALPHABET = (
    b"abcdefghijklmnopqrstuvwxyz"
    b"ABCDEFGHIJKLMNOPQRSTUVWXYZ"
    b"0123456789!@#0^&*();:<>,. []{}"
)  # fmt: skip


@pytest.mark.skipif(sys.version_info < (3, 15), reason="requires CPython 3.15+")
@pytest.mark.parametrize(
    "altchars",
    [
        b"invalid",
        b"tooooooooo_looooooong",
        b"\x00\x01x00\x01",
    ],
)
def test_invalid_altchars(altchars: bytes) -> None:
    with pytest.raises(ValueError, match="invalid altchars:"):
        base64.b64encode(b"lava-sh", altchars)

    with pytest.raises(ValueError, match="invalid altchars:"):
        base64_rs.b64encode(b"lava-sh", altchars)


@pytest.mark.skipif(sys.version_info < (3, 15), reason="requires CPython 3.15+")
def test_invalid_wrapcol() -> None:
    with pytest.raises(ValueError, match="Cannot convert negative int"):
        base64.b64encode(b"lava-sh", wrapcol=-1)

    with pytest.raises(ValueError, match="wrapcol must be >= 0"):
        base64_rs.b64encode(b"lava-sh", wrapcol=-1)


@pytest.mark.parametrize(
    "b",
    [
        b"", b"a", b"ab", b"abc",
        b"\x00", b"lava-sh",
        ALPHABET,
    ],
)  # fmt: skip
def test_b64encode(b: bytes) -> None:
    assert base64.b64encode(b) == base64_rs.b64encode(b)


@pytest.mark.parametrize(
    "altchars",
    [
        b"*$",
        bytearray(b"*$"),
        memoryview(b"*$"),
        array.array("B", b"*$"),
    ],
)
def test_b64encode_with_altchars(altchars: base64_rs._base64_rs.ReadableBuffer) -> None:
    data = b"\xd3V\xbeo\xf7\x1d"
    expected = b"01a*b$cd"

    assert base64.b64encode(data, altchars=altchars) == expected
    assert (
        base64.b64encode(data, altchars=altchars) ==
        base64_rs.b64encode(data, altchars=altchars)
    )  # fmt: skip
