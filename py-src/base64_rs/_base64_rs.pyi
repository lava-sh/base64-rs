import abc
import builtins
import sys
from typing import TypeAlias

__version__: str

if sys.version_info >= (3, 12):
    from collections.abc import Buffer
else:
    class Buffer(abc.ABC): ...

    Buffer.register(memoryview)
    Buffer.register(bytearray)
    Buffer.register(bytes)

ReadableBuffer: TypeAlias = Buffer

def _b64encode(
    s: ReadableBuffer,
    altchars: ReadableBuffer | None = None,
    *,
    padded: builtins.bool = True,
    wrapcol: builtins.int = 0,
) -> builtins.bytes: ...

def _b64encode_scalar(
    s: ReadableBuffer,
    altchars: ReadableBuffer | None = None,
    *,
    padded: builtins.bool = True,
    wrapcol: builtins.int = 0,
) -> builtins.bytes: ...

def _b64encode_ssse3(
    s: ReadableBuffer,
    altchars: ReadableBuffer | None = None,
    *,
    padded: builtins.bool = True,
    wrapcol: builtins.int = 0,
) -> builtins.bytes: ...

def _b64encode_avx2(
    s: ReadableBuffer,
    altchars: ReadableBuffer | None = None,
    *,
    padded: builtins.bool = True,
    wrapcol: builtins.int = 0,
) -> builtins.bytes: ...

def _b64encode_avx512(
    s: ReadableBuffer,
    altchars: ReadableBuffer | None = None,
    *,
    padded: builtins.bool = True,
    wrapcol: builtins.int = 0,
) -> builtins.bytes: ...

def _b64decode(
    s: builtins.str | ReadableBuffer,
    altchars: builtins.str | ReadableBuffer | None = None,
    validate: builtins.bool = ...,
    *,
    padded: builtins.bool = True,
    ignorechars: ReadableBuffer = ...,
    canonical: builtins.bool = False,
) -> builtins.bytes: ...

def _standard_b64encode(s: ReadableBuffer) -> builtins.bytes: ...
def _standard_b64decode(s: builtins.str | ReadableBuffer) -> builtins.bytes: ...

def _urlsafe_b64encode(
    s: ReadableBuffer,
    *,
    padded: builtins.bool = True,
) -> builtins.bytes: ...

def _urlsafe_b64decode(
    s: builtins.str | ReadableBuffer,
    *,
    padded: bool = False,
) -> builtins.bytes: ...
