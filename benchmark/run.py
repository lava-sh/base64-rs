import argparse
import base64
import gc
import statistics
import time
from collections.abc import Callable
from typing import TypeAlias

import base64_rs
import pybase64
from rich import box
from rich.console import Console
from rich.table import Table
from rich.text import Text

BYTE_SIZES = (
    1_000,
    512_000,
    1_000_000,
    32_000_000,
    64_000_000,
    256_000_000,
    512_000_000,
)

TARGET_SAMPLE_NS = 25_000_000

Benchmark: TypeAlias = tuple[str, Callable[[bytes], bytes]]


def samples_for(size: int, requested: int) -> int:
    if size <= 1_000_000:
        minimum = 21
    elif size <= 64_000_000:
        minimum = 11
    elif size <= 512_000_000:
        minimum = 7
    else:
        minimum = 3
    return max(minimum, requested)


def calls_per_sample(function: Callable[[bytes], bytes], data: bytes) -> int:
    start = time.perf_counter_ns()
    function(data)
    elapsed = max(1, time.perf_counter_ns() - start)
    return max(1, TARGET_SAMPLE_NS // elapsed)


def elapsed_per_call(
    function: Callable[[bytes], bytes],
    data: bytes,
    calls: int,
) -> float:
    start = time.perf_counter_ns()
    for _ in range(calls):
        function(data)
    return (time.perf_counter_ns() - start) / calls


def display_size(size: int) -> str:
    if size >= 1_000_000_000:
        return f"{size / 1_000_000_000:g}GB"
    if size >= 1_000_000:
        return f"{size / 1_000_000:g}MB"
    return f"{size / 1_000:g}KB"


def create_table(headers: list[str]) -> Table:
    table = Table(border_style="bright_black", box=box.SQUARE)
    table.add_column(Text(headers[0], justify="center"), justify="left")
    for header in headers[1:]:
        table.add_column(Text(header, justify="center"), justify="right")
    return table


def benchmark_size(
    fns: tuple[Benchmark, ...],
    data: bytes,
    requested_samples: int,
) -> dict[str, float]:
    calls = {name: calls_per_sample(function, data) for name, function in fns}
    samples = samples_for(len(data), requested_samples)
    values: dict[str, list[float]] = {name: [] for name, _ in fns}

    for sample in range(samples):
        for offset in range(len(fns)):
            name, function = fns[(sample + offset) % len(fns)]
            values[name].append(elapsed_per_call(function, data, calls[name]))

    return {name: statistics.median(value) / 1_000_000 for name, value in values.items()}


def run_benchmark(
    requested_samples: int,
    title: str,
    fns: tuple[Benchmark, ...],
) -> None:
    console = Console()
    console.print(f"\n[bold]{title}[/bold]")
    table = create_table(["Size", *(name for name, _ in fns)])

    for size in BYTE_SIZES:
        results = benchmark_size(fns, bytes(size), requested_samples)
        baseline = results[fns[0][0]]
        fastest = min(results.values())
        row = [display_size(size)]

        for name, _ in fns:
            elapsed = results[name]
            value = f"{elapsed:.4f} ms ({baseline / elapsed:.1f}x)"
            if elapsed == fastest:
                value = f"[green]{value}[/green]"
            row.append(value)

        table.add_row(*row)

    console.print(table)


def run_basic(runs: int) -> None:
    run_benchmark(runs, "Basic: b64encode(b'hello world')", (
        ("std", base64.b64encode),
        ("pybase64", pybase64.b64encode),
        ("base64_rs", base64_rs.b64encode),
        ("base64_rs (no simd)", base64_rs._b64encode_scalar),
    ))  # fmt: skip


def run_with_altchars(runs: int) -> None:
    run_benchmark(runs, "With altchars: b64encode(b'hello world', altchars=b'-_')", (
        ("std", lambda b: base64.b64encode(b, altchars=b"-_")),
        ("pybase64", lambda b: pybase64.b64encode(b, altchars=b"-_")),
        ("base64_rs", lambda b: base64_rs.b64encode(b, altchars=b"-_")),
        ("base64_rs (no simd)", lambda b: base64_rs._b64encode_scalar(b, altchars=b"-_")),
    ))  # fmt: skip


def run_with_padded(runs: int) -> None:
    run_benchmark(runs, "With padded=False: b64encode(b'hello world', padded=False)", (
        ("std", lambda b: base64.b64encode(b, padded=False)),
        ("pybase64", lambda b: pybase64.b64encode(b, padded=False)),
        ("base64_rs", lambda b: base64_rs.b64encode(b, padded=False)),
        ("base64_rs (no simd)", lambda b: base64_rs._b64encode_scalar(b, padded=False)),
    ))  # fmt: skip


def run_with_wrapcol(runs: int) -> None:
    run_benchmark(runs, "With wrapcol=76: b64encode(b'hello world', wrapcol=76)", (
        ("std", lambda b: base64.b64encode(b, wrapcol=76)),
        ("pybase64", lambda b: pybase64.b64encode(b, wrapcol=76)),
        ("base64_rs", lambda b: base64_rs.b64encode(b, wrapcol=76)),
        ("base64_rs (no simd)", lambda b: base64_rs._b64encode_scalar(b, wrapcol=76)),
    ))  # fmt: skip


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--runs", type=int, default=0)
    parser.add_argument(
        "--type",
        choices=("basic", "with_altchars", "with_padded", "with_wrapcol", "full"),
        default="full",
    )
    args = parser.parse_args()

    benchmarks = {
        "basic": run_basic,
        "with_altchars": run_with_altchars,
        "with_padded": run_with_padded,
        "with_wrapcol": run_with_wrapcol,
    }

    gc_was_enabled = gc.isenabled()
    gc.disable()
    try:
        if args.type == "full":
            for benchmark in benchmarks.values():
                benchmark(args.runs)
        else:
            benchmarks[args.type](args.runs)
    finally:
        if gc_was_enabled:
            gc.enable()


if __name__ == "__main__":
    main()
