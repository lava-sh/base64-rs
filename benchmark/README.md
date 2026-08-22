# To run the benchmarks

## Create and activate a virtual environment

<p>
  <span style="white-space: nowrap;">
    <img
      src="https://thesvg.org/icons/linux/default.svg"
      alt="linux"
      height="14"
    />
    Linux /
    <picture>
      <source
        media="(prefers-color-scheme: dark)"
        srcset="https://thesvg.org/icons/apple/default.svg"
      />
      <img
        src="https://thesvg.org/icons/apple/mono.svg"
        alt="macos"
        height="14"
      />
    </picture>
    MacOS:
  </span>
</p>

```bash
python3 -m venv .venv
# or uv venv .venv --seed

source .venv/bin/activate
```

<p>
  <img
    src="https://thesvg.org/icons/windows/default.svg"
    alt="windows"
    height="14"
  />
  Windows:
</p>

```bash
py -m venv .venv
# or uv venv .venv --seed

.venv\scripts\activate
```

## Install benchmark dependencies

<p>
  <img
    src="https://thesvg.org/icons/python/default.svg"
    alt="Python"
    height="14"
  />
  Using <a href="https://github.com/pypa/pip">pip</a>:
</p>

```bash
pip install . --group bench
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
uv pip install . --group bench
```

## Run `benchmark/run.py`

```bash
python benchmark/run.py --runs 3 --type full
```

## Results

Windows 10 · [Intel® Core™ i5-11300H](https://www.intel.com/content/www/us/en/products/sku/196656/intel-core-i511300h-processor-8m-cache-up-to-4-40-ghz-with-ipu/specifications.html)

```console
❯ python benchmark/run.py --runs 1 --type full
Basic: b64encode(b'hello world')
┌───────┬────────────────────┬────────────────────┬────────────────────┬─────────────────────┐
│ Size  │        std         │      pybase64      │     base64_rs      │ base64_rs (no simd) │
├───────┼────────────────────┼────────────────────┼────────────────────┼─────────────────────┤
│ 1KB   │   0.0007 ms (1.0x) │   0.0004 ms (2.1x) │   0.0002 ms (3.4x) │    0.0004 ms (1.8x) │
│ 512KB │   0.2633 ms (1.0x) │   0.0460 ms (5.7x) │  0.0118 ms (22.4x) │    0.0988 ms (2.7x) │
│ 1MB   │   0.8060 ms (1.0x) │   0.4531 ms (1.8x) │   0.3963 ms (2.0x) │    0.5199 ms (1.6x) │
│ 32MB  │  25.7500 ms (1.0x) │  14.2107 ms (1.8x) │  12.4099 ms (2.1x) │   15.8952 ms (1.6x) │
│ 64MB  │  50.3747 ms (1.0x) │  25.8808 ms (1.9x) │  25.6143 ms (2.0x) │   30.3312 ms (1.7x) │
│ 256MB │ 200.1055 ms (1.0x) │ 119.1325 ms (1.7x) │ 107.7002 ms (1.9x) │  130.2308 ms (1.5x) │
│ 512MB │ 401.8704 ms (1.0x) │ 208.0739 ms (1.9x) │ 209.2670 ms (1.9x) │  253.9221 ms (1.6x) │
└───────┴────────────────────┴────────────────────┴────────────────────┴─────────────────────┘

With altchars: b64encode(b'hello world', altchars=b'-_')
┌───────┬────────────────────┬────────────────────┬────────────────────┬─────────────────────┐
│ Size  │        std         │      pybase64      │     base64_rs      │ base64_rs (no simd) │
├───────┼────────────────────┼────────────────────┼────────────────────┼─────────────────────┤
│ 1KB   │   0.0010 ms (1.0x) │   0.0017 ms (0.6x) │   0.0003 ms (3.2x) │    0.0005 ms (1.9x) │
│ 512KB │   0.2604 ms (1.0x) │   0.6598 ms (0.4x) │  0.0121 ms (21.5x) │    0.0980 ms (2.7x) │
│ 1MB   │   0.8017 ms (1.0x) │   1.5949 ms (0.5x) │   0.3997 ms (2.0x) │    0.4895 ms (1.6x) │
│ 32MB  │  25.2932 ms (1.0x) │  52.5511 ms (0.5x) │  12.2050 ms (2.1x) │   15.0851 ms (1.7x) │
│ 64MB  │  51.0873 ms (1.0x) │ 107.3928 ms (0.5x) │  24.5995 ms (2.1x) │   30.1465 ms (1.7x) │
│ 256MB │ 205.0496 ms (1.0x) │ 423.6367 ms (0.5x) │  98.9091 ms (2.1x) │  120.7559 ms (1.7x) │
│ 512MB │ 408.0598 ms (1.0x) │ 858.1952 ms (0.5x) │ 209.4071 ms (1.9x) │  266.3051 ms (1.5x) │
└───────┴────────────────────┴────────────────────┴────────────────────┴─────────────────────┘

With padded=False: b64encode(b'hello world', padded=False)
┌───────┬────────────────────┬────────────────────┬────────────────────┬─────────────────────┐
│ Size  │        std         │      pybase64      │     base64_rs      │ base64_rs (no simd) │
├───────┼────────────────────┼────────────────────┼────────────────────┼─────────────────────┤
│ 1KB   │   0.0008 ms (1.0x) │   0.0006 ms (1.4x) │   0.0003 ms (2.8x) │    0.0005 ms (1.7x) │
│ 512KB │   0.3684 ms (1.0x) │   0.1722 ms (2.1x) │   0.1103 ms (3.3x) │    0.1922 ms (1.9x) │
│ 1MB   │   1.0314 ms (1.0x) │   0.5938 ms (1.7x) │   0.5603 ms (1.8x) │    0.6651 ms (1.6x) │
│ 32MB  │  29.4041 ms (1.0x) │  18.2171 ms (1.6x) │  17.4168 ms (1.7x) │   20.7782 ms (1.4x) │
│ 64MB  │  62.6156 ms (1.0x) │  37.5545 ms (1.7x) │  32.3833 ms (1.9x) │   41.4839 ms (1.5x) │
│ 256MB │ 253.2510 ms (1.0x) │ 138.4930 ms (1.8x) │ 131.4053 ms (1.9x) │  155.4846 ms (1.6x) │
│ 512MB │ 421.4952 ms (1.0x) │ 230.2350 ms (1.8x) │ 233.5092 ms (1.8x) │  287.5512 ms (1.5x) │
└───────┴────────────────────┴────────────────────┴────────────────────┴─────────────────────┘

With wrapcol=76: b64encode(b'hello world', wrapcol=76)
┌───────┬────────────────────┬────────────────────┬────────────────────┬─────────────────────┐
│ Size  │        std         │      pybase64      │     base64_rs      │ base64_rs (no simd) │
├───────┼────────────────────┼────────────────────┼────────────────────┼─────────────────────┤
│ 1KB   │   0.0009 ms (1.0x) │   0.0021 ms (0.4x) │   0.0006 ms (1.5x) │    0.0006 ms (1.4x) │
│ 512KB │   0.3639 ms (1.0x) │   0.8430 ms (0.4x) │   0.2095 ms (1.7x) │    0.2071 ms (1.8x) │
│ 1MB   │   1.0721 ms (1.0x) │   1.9599 ms (0.5x) │   0.7160 ms (1.5x) │    0.7169 ms (1.5x) │
│ 32MB  │  35.7743 ms (1.0x) │  61.3933 ms (0.6x) │  22.9023 ms (1.6x) │   22.6066 ms (1.6x) │
│ 64MB  │  69.9220 ms (1.0x) │ 131.2019 ms (0.5x) │  46.5069 ms (1.5x) │   44.8281 ms (1.6x) │
│ 256MB │ 316.0486 ms (1.0x) │ 502.5255 ms (0.6x) │ 185.9391 ms (1.7x) │  186.1877 ms (1.7x) │
│ 512MB │ 555.5429 ms (1.0x) │ 988.1109 ms (0.6x) │ 341.8665 ms (1.6x) │  341.7833 ms (1.6x) │
└───────┴────────────────────┴────────────────────┴────────────────────┴─────────────────────┘
```
