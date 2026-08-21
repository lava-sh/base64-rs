# ruff: noqa: E501
import argparse
import base64
import os
import time
from collections.abc import Callable

import base64_rs
import pybase64
from rich import box
from rich.console import Console
from rich.table import Table
from rich.text import Text

ByteSizes = [
    1,
    512,
    1024,
    32 * 1024,
    64 * 1024,
    256 * 1024,
    512 * 1024,
    1024 * 1024,
]


def generate_rand_bytes(size_kb: int) -> bytes:
    return os.urandom(size_kb * 1024)


def bench(func: Callable[[bytes], bytes], data: bytes, runs: int) -> float:
    start = time.perf_counter()

    for _ in range(runs):
        func(data)

    return (time.perf_counter() - start) / runs * 1000


def create_table(headers: list[str]) -> Table:
    table = Table(border_style="bright_black", box=box.SQUARE)
    table.add_column(Text(headers[0], justify="center"), justify="center")

    for header in headers[1:]:
        table.add_column(Text(header, justify="center"), justify="right")

    return table


def run_benchmark(
    runs: int,
    title: str,
    fns: list[tuple[str, Callable[[bytes], bytes]]],
) -> None:
    console = Console()
    console.print(f"\n[bold]{title}[/bold]")

    table = create_table(["Size", *(name for name, _ in fns)])

    for size in ByteSizes:
        data = generate_rand_bytes(size)
        results = [bench(func, data, runs) for _, func in fns]

        baseline = results[0]
        fastest = min(results)

        size_str = f"{size // 1024}MB" if size >= 1024 else f"{size}KB"

        row = [size_str]

        for time_ms in results:
            speedup = baseline / time_ms
            value = f"{time_ms:.4f} ms ({speedup:.1f}x)"

            if time_ms == fastest:
                value = f"[green]{value}[/green]"

            row.append(value)

        table.add_row(*row)

    console.print(table)


def run_basic(runs: int) -> None:
    run_benchmark(
        runs,
        "Basic: b64encode(b'hello world')",
        [
            ("std", base64.b64encode),
            ("pybase64", pybase64.b64encode),
            ("base64_rs", base64_rs.b64encode),
            ("base64_rs (no simd)", base64_rs._b64encode_scalar),
        ],
    )  # fmt: skip


def run_with_altchars(runs: int) -> None:
    run_benchmark(
        runs,
        "With altchars: b64encode(b'hello world', altchars=b'-_')",
        [
            ("std", lambda b: base64.b64encode(b, altchars=b"-_")),
            ("pybase64", lambda b: pybase64.b64encode(b, altchars=b"-_")),
            ("base64_rs", lambda b: base64_rs.b64encode(b, altchars=b"-_")),
            ("base64_rs (no simd)", lambda b: base64_rs._b64encode_scalar(b, altchars=b"-_")),
        ],
    )  # fmt: skip


def run_with_padded(runs: int) -> None:
    run_benchmark(
        runs,
        "With padded=False: b64encode(b'hello world', padded=False)",
        [
            ("std", lambda b: base64.b64encode(b, padded=False)),
            ("pybase64", lambda b: pybase64.b64encode(b, padded=False)),
            ("base64_rs", lambda b: base64_rs.b64encode(b, padded=False)),
            ("base64_rs (no simd)", lambda b: base64_rs._b64encode_scalar(b, padded=False)),
        ],
    )  # fmt: skip


def run_with_wrapcol(runs: int) -> None:
    run_benchmark(
        runs,
        "With wrapcol=76: b64encode(b'hello world', wrapcol=76)",
        [
            ("std", lambda b: base64.b64encode(b, wrapcol=76)),
            ("pybase64", lambda b: pybase64.b64encode(b, wrapcol=76)),
            ("base64_rs", lambda b: base64_rs.b64encode(b, wrapcol=76)),
            ("base64_rs (no simd)", lambda b: base64_rs._b64encode_scalar(b, wrapcol=76)),
        ],
    )  # fmt: skip


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--runs", type=int, default=20)
    parser.add_argument(
        "--type",
        choices=[
            "basic",
            "with_altchars",
            "with_padded",
            "with_wrapcol",
            "full",
        ],
        default="full",
    )
    args = parser.parse_args()

    benchmark_types = {
        "basic": run_basic,
        "with_altchars": run_with_altchars,
        "with_padded": run_with_padded,
        "with_wrapcol": run_with_wrapcol,
    }

    if args.type == "full":
        for benchmark in benchmark_types.values():
            benchmark(args.runs)
    else:
        benchmark_types[args.type](args.runs)


if __name__ == "__main__":
    main()
