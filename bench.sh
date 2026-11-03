#!/usr/bin/env bash
# Usage: ./bench.sh [--cold] [--perf] [csv]
#   --cold  also benchmark with the page cache dropped before each run (needs sudo)
#   --perf  also print perf stat counters (needs kernel.perf_event_paranoid <= 1)
set -euo pipefail
cd "$(dirname "$0")"

cold=0 perf=0 data=data.csv
for arg in "$@"; do
    case $arg in
        --cold) cold=1 ;;
        --perf) perf=1 ;;
        -*) echo "unknown option: $arg" >&2; exit 2 ;;
        *) data=$arg ;;
    esac
done

for tool in hyperfine clang++ cargo; do
    command -v "$tool" >/dev/null || { echo "missing: $tool" >&2; exit 1; }
done

[[ -f $data ]] || python3 gen_csv.py 1000000 "$data"

echo "== build"
clang++ -std=c++20 -O3 -march=native -DNDEBUG cpp/csv_fast.cpp -o cpp/csv_fast
RUSTFLAGS="-C target-cpu=native" cargo build --release --quiet --manifest-path rust/Cargo.toml

names=(cpp rust)
bins=(cpp/csv_fast rust/target/release/csv_naive)

echo "== output check"
ref=$("${bins[0]}" "$data")
for i in "${!bins[@]}"; do
    if [[ $("${bins[$i]}" "$data") != "$ref" ]]; then
        echo "${names[$i]} output differs from ${names[0]}" >&2
        diff <(echo "$ref") <("${bins[$i]}" "$data") >&2 || true
        exit 1
    fi
done
echo "identical"

cmds=()
for i in "${!bins[@]}"; do cmds+=(-n "${names[$i]}" "${bins[$i]} $data"); done

echo "== warm cache"
hyperfine -N --warmup 5 --runs 50 "${cmds[@]}"

if ((cold)); then
    echo "== cold cache"
    sudo -v
    hyperfine -N --runs 10 --prepare 'sudo sh -c "sync; echo 3 > /proc/sys/vm/drop_caches"' "${cmds[@]}"
fi

echo "== peak memory"
for i in "${!bins[@]}"; do
    /usr/bin/time -f "${names[$i]}: %M KB peak RSS" "${bins[$i]}" "$data" >/dev/null
done

if ((perf)); then
    echo "== perf stat"
    for i in "${!bins[@]}"; do
        echo "-- ${names[$i]}"
        perf stat -r 20 -e cycles,instructions,branch-misses,page-faults "${bins[$i]}" "$data" 2>&1 >/dev/null |
            grep -E 'cycles|instructions|branch-misses|page-faults|elapsed'
    done
fi
