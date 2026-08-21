__all__ = (
    "__version__",
    "_b64encode_avx2",
    "_b64encode_avx512",
    "_b64encode_scalar",
    "_b64encode_ssse3",
    "b64encode",
)

from ._base64_rs import (
    __version__,
    _b64encode as b64encode,
    _b64encode_avx2,
    _b64encode_avx512,
    _b64encode_scalar,
    _b64encode_ssse3,
)
