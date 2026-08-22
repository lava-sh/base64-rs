<!-- rumdl-disable MD036 MD041 -->
<div align="center">

# base64-rs

_High-performance base64 encoder/decoder written in Rust🦅_
<!-- rumdl-enable MD036 MD041 -->

[![PyPI version][pypi-version-badge]][pypi]
[![PyPI downloads][pypi-downloads-badge]][pypistats]
[![PyPI requires python][pypi-requires-python-badge]][pypi]

<a href="https://github.com/lava-sh/base64-rs/actions?query=branch%3Amain"><picture><source media="(prefers-color-scheme: dark)" srcset="https://shieldcn.dev/github/ci/lava-sh/base64-rs.svg?variant=outline&font=geist-mono&size=xs&animate=pulse&mode=dark"><img alt="CI" src="https://shieldcn.dev/github/ci/lava-sh/base64-rs.svg?variant=outline&font=geist-mono&size=xs&animate=pulse&mode=light"></picture></a>
<a href="https://github.com/lava-sh/base64-rs/commits/main"><picture><source media="(prefers-color-scheme: dark)" srcset="https://shieldcn.dev/github/last-commit/lava-sh/base64-rs.svg?variant=outline&font=geist-mono&size=xs&mode=dark"><img alt="Last Commit" src="https://shieldcn.dev/github/last-commit/lava-sh/base64-rs.svg?variant=outline&font=geist-mono&size=xs&mode=light"></picture></a>
<a href="https://github.com/lava-sh/base64-rs/blob/main/UNLICENSE"><picture><source media="(prefers-color-scheme: dark)" srcset="https://shieldcn.dev/github/lava-sh/base64-rs/license.svg?variant=outline&font=geist-mono&size=xs&mode=dark"><img alt="License" src="https://shieldcn.dev/github/lava-sh/base64-rs/license.svg?variant=outline&font=geist-mono&size=xs&mode=light"></picture></a>

</div>

## Features

* High-performance base64 encoder/decoder

* Runtime SIMD dispatch with support for:
  * AVX-512F, AVX-512VL, AVX-512VBMI, AVX2, AVX
  * SSE4.2, SSE4.1, SSSE3
  * NEON64, NEON32

* Drop-in replacement for CPython [base64][cpython-base64] (see: [below](https://github.com/lava-sh/base64-rs#compatibility-with-cpython-base64))

## Installation

<p>
  <img
    src="https://thesvg.org/icons/python/default.svg"
    alt="Python"
    height="14"
  />
  Using <a href="https://github.com/pypa/pip">pip</a>:
</p>

```bash
pip install base64-rs
```

<p>
  <img
    src="https://thesvg.org/icons/uv/default.svg"
    alt="uv"
    height="14"
  />
  Using <a href="https://github.com/astral-sh/uv">uv</a>:
</p>

```bash
uv pip install base64-rs
```

<p>
  <img
    src="https://thesvg.org/icons/poetry/default.svg"
    alt="Poetry"
    height="14"
  />
  Using <a href="https://github.com/python-poetry/poetry">poetry</a>:
</p>

```bash
poetry add base64-rs
```

## Examples

```python
import base64_rs

print(base64_rs.b64encode(b"lava-sh")) # b'bGF2YS1zaA=='
```

## Compatibility with CPython [base64][cpython-base64]

`base64-rs` is a drop-in replacement for CPython's [base64][cpython-base64]
lin on Python 3.15+. Its compatible functions use the same call signatures,
parameters, defaults, and return values, so existing imports can be replaced
without changing call sites.

```py
import base64_rs as base64

print(base64.b64encode(b"lava-sh"))           # b'bGF2YS1zaA=='
print(base64.standard_b64encode(b"\xfb\xff")) # b'+/8='
print(base64.urlsafe_b64encode(b"\xfb\xff"))  # b'-_8='
```

## References

### Papers

* 📗 [Base64 encoding and decoding at almost the speed of a memory copy](https://arxiv.org/pdf/1910.05109)
* 📗 [AVX512F base64 coding and decoding](http://0x80.pl/notesen/2016-09-17-avx512-foundation-base64.html)
* 📗 [Base64 encoding & decoding using AVX512BW instructions](http://0x80.pl/notesen/2016-04-03-avx512-base64.html)
* 📗 [Base64 encoding with SIMD instructions](http://0x80.pl/notesen/2016-01-12-sse-base64-encoding.html)
* 📗 [Base64 decoding with SIMD instructions](http://0x80.pl/notesen/2016-01-17-sse-base64-decoding.html)

### GitHub repos

* 🐙 [WojciechMula/base64simd](https://github.com/WojciechMula/base64simd)
* 🐙 [WojciechMula/base64-avx512](https://github.com/WojciechMula/base64-avx512)
* 🐙 [aklomp/base64](https://github.com/aklomp/base64)
* 🐙 [Nugine/simd](https://github.com/Nugine/simd)
* 🐙 [BLAKE3-team/BLAKE3](https://github.com/BLAKE3-team/BLAKE3)
* 🐙 [simdutf/simdutf](https://github.com/simdutf/simdutf)

<div align="center">

## Contributors

[![lava-sh/base64-rs contributors][contributors-badge]][github-contributors]

</div>

[cpython-base64]: https://docs.python.org/3.15/library/base64.html

[github-contributors]: https://github.com/lava-sh/base64-rs/graphs/contributors

[pypi]: https://pypi.org/project/base64-rs
[pypistats]: https://pypistats.org/packages/base64-rs

[pypi-version-badge]: https://shieldcn.dev/badge/dynamic/json.svg?url=https%3A%2F%2Fpypi.org%2Fpypi%2Fbase64-rs%2Fjson&query=%24.info.version&variant=branded&size=xs&mode=light&logo=python&label=pypi+version
[pypi-downloads-badge]: https://shieldcn.dev/pypi/dm/base64-rs.svg?variant=branded&size=xs&logo=python&logoColor=ffffff
[pypi-requires-python-badge]: https://shieldcn.dev/badge/dynamic/json.svg?url=https%3A%2F%2Fpypi.org%2Fpypi%2Fbase64-rs%2Fjson&query=%24.info.requires_python&size=xs&mode=light&logo=python&logoColor=ffffff&label=requires+python&color=3775A9

[contributors-badge]: https://shieldcn.dev/contributors/lava-sh/base64-rs.svg?title=false&theme=slate&size=80&bots=true&titleAlign=center&mode=light&font=geist&border=false&image=https%3A%2F%2Fimages.wallpaperscraft.ru%2Fimage%2Fsingle%2Foblaka_nebo_ogni_1647475_3840x2400.jpg&overlay=0.3
