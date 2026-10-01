#!/usr/bin/env bash
# Regenerates asm-compare/asm/: the release x86-64 assembly of every
# `hand_*` and `groups_*` function, one file per module, with assembler
# directives stripped.
set -euo pipefail

root="$(cd "$(dirname "$0")/.." && pwd)"
out="$root/asm-compare/asm"

cargo rustc --manifest-path "$root/Cargo.toml" -p asm-compare --lib \
    --release -- --emit asm -C codegen-units=1 \
    -C llvm-args=-x86-asm-syntax=intel

asm="$(ls -t "$root"/target/release/deps/asm_compare-*.s | head -n 1)"
mkdir -p "$out"

for module in hand groups; do
    awk -v prefix="${module}_" '
        FNR == NR {
            if ($0 ~ /^[A-Za-z_][A-Za-z0-9_]*:$/) {
                name = substr($0, 1, length($0) - 1)
                body[name] = ""
            } else if ($0 ~ /^\.Lfunc_end/) {
                name = ""
            } else if (name != "" && $0 !~ /^[[:space:]]*[.#]/ \
                    && $0 !~ /^\.L/) {
                body[name] = body[name] $0 "\n"
            }
            next
        }
        /^[A-Za-z_][A-Za-z0-9_]*:$/ {
            name = substr($0, 1, length($0) - 1)
            if (index(name, prefix) == 1) {
                printf "%s%s:\n%s", sep, name, body[name]
                sep = "\n"
            }
        }
        /^[A-Za-z_][A-Za-z0-9_]* = [A-Za-z_][A-Za-z0-9_]*$/ {
            if (index($1, prefix) == 1) {
                printf "%s%s:  # alias of %s, LLVM merged identical IR\n%s", \
                    sep, $1, $3, body[$3]
                sep = "\n"
            }
        }
    ' "$asm" "$asm" > "$out/$module.s"
done

echo "wrote $out/hand.s and $out/groups.s"
