#!/usr/bin/env bash
# B4 isolated process-cost measurements. Invoke explicitly; never resident.
set -euo pipefail
repo="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd -P)"
exe="${1:-$repo/target/release/adaptive-agent}"
runs="${2:-10}"
warmup="${3:-2}"
if [[ ! -x "$exe" ]]; then echo 'Executable unavailable' >&2; exit 2; fi
if [[ ! "$runs" =~ ^[0-9]+$ ]] || (( 10#$runs < 5 || 10#$runs > 30 )); then echo 'Runs must be 5..30' >&2; exit 2; fi
if [[ ! "$warmup" =~ ^[0-9]+$ ]] || (( 10#$warmup < 1 || 10#$warmup > 5 )); then echo 'Warmup must be 1..5' >&2; exit 2; fi
if [[ ! -x /usr/bin/time ]]; then echo 'GNU time not installed; metrics unavailable' >&2; exit 2; fi
command -v awk >/dev/null || { echo 'awk missing' >&2; exit 2; }
command -v sort >/dev/null || { echo 'sort missing' >&2; exit 2; }
export LC_ALL=C
scratch="$(mktemp -d)"
trap 'rm -rf -- "$scratch"' EXIT

for mode in memory cpu power; do
    printf '=== B4 LINUX %s: %s warmups + %s measured ===\n' "$mode" "$warmup" "$runs"
    for ((i=0; i<10#$warmup; i++)); do
        "$exe" "--telemetry-$mode" >/dev/null
    done
    : > "$scratch/$mode"
    for ((i=0; i<10#$runs; i++)); do
        # GNU time reports elapsed, user and system CPU seconds, and peak RSS KiB.
        /usr/bin/time -f '%e %U %S %M %x' -o "$scratch/timing" \
            "$exe" "--telemetry-$mode" >/dev/null
        read -r elapsed user system peak exit_code < "$scratch/timing"
        if [[ "$exit_code" != 0 ]]; then echo 'Collector failed' >&2; exit 1; fi
        printf '%s %s %s %s\n' "$elapsed" "$user" "$system" "$peak" >> "$scratch/$mode"
    done
    awk '{ if ($1 < 0 || $2 < 0 || $3 < 0 || $4 <= 0) exit 1 }' "$scratch/$mode" || { echo 'Invalid metrics' >&2; exit 1; }
    awk '{ printf "%.3f %.3f %.3f\n",$1*1000,($2+$3)*1000,$4/1024 }' "$scratch/$mode" > "$scratch/$mode.cols"
    n="$(wc -l < "$scratch/$mode.cols")"
    p50=$(( (n + 1) / 2 ))
    p95=$(( (95 * n + 99) / 100 ))
    wall50="$(awk '{print $1}' "$scratch/$mode.cols" | sort -n | sed -n "${p50}p")"
    wall95="$(awk '{print $1}' "$scratch/$mode.cols" | sort -n | sed -n "${p95}p")"
    cpu95="$(awk '{print $2}' "$scratch/$mode.cols" | sort -n | sed -n "${p95}p")"
    rss95="$(awk '{print $3}' "$scratch/$mode.cols" | sort -n | sed -n "${p95}p")"
    printf 'mode=%s count=%s elapsed_ms_p50=%s elapsed_ms_p95=%s cpu_ms_p95=%s peak_rss_mib_p95=%s\n' \
        "$mode" "$n" "$wall50" "$wall95" "$cpu95" "$rss95"
done
printf '%s\n' 'B4 Linux: includes launch/output, not energy; CPU time is not CPU percent.'
printf '%s\n' 'B4 Linux metrics: PASS'
