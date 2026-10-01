#!/usr/bin/env bash
# Runs every ignored test in `tests/miri_ub.rs` alone under Miri, and
# fails unless each one reports undefined behaviour.
set -euo pipefail

args=(-q -p typestate-groups-macros --test miri_ub -- --ignored)
tests=$(cargo +nightly miri test "${args[@]}" --list | sed -n 's/: test$//p')
if [ -z "$tests" ]; then
    echo "add #[ignore] tests to tests/miri_ub.rs"
    exit 1
fi

status=0
for test in $tests; do
    output=$(cargo +nightly miri test "${args[@]}" --exact "$test" 2>&1 || true)
    if grep -q "Undefined Behavior" <<<"$output"; then
        echo "ub as expected: $test"
    else
        echo "make $test trigger undefined behaviour under Miri:"
        echo "$output"
        status=1
    fi
done
exit $status
