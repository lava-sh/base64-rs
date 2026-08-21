__all__ = (
    "__version__",
    "_b64encode_scalar",
    "b64encode",
    "standard_b64encode",
    "urlsafe_b64encode",
)

from ._base64_rs import (
    __version__,
    _b64encode as b64encode,
    _b64encode_scalar,
    _standard_b64encode as standard_b64encode,
    _urlsafe_b64encode as urlsafe_b64encode,
)

try:  # noqa: RUF067
    from ._base64_rs import (
        _b64encode_avx2,
        _b64encode_avx512,
        _b64encode_ssse3,
    )
except ImportError:
    pass
else:
    __all__ += (  # type: ignore[assignment]
        "_b64encode_avx2",
        "_b64encode_avx512",
        "_b64encode_ssse3",
    )
