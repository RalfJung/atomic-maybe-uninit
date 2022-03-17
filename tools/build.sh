#!/bin/bash
set -euo pipefail
IFS=$'\n\t'
cd "$(dirname "$0")"/..

trap -- 'exit 0' SIGINT

default_targets=(
    # x86_64
    # rustc --print target-list | grep -E '^x86_64' | grep -E '(-unknown-linux|-none)'
    x86_64-unknown-linux-gnu
    # x86_64 X32 ABI
    x86_64-unknown-linux-gnux32

    # x86
    # rustc --print target-list | grep -E '^i.86' | grep -E '(-unknown-linux|-none)'
    i586-unknown-linux-gnu
    i686-unknown-linux-gnu

    # aarch64
    # rustc --print target-list | grep -E '^aarch64' | grep -E '(-unknown-linux|-none)'
    aarch64-unknown-linux-gnu
    # aarch64 big endian
    aarch64_be-unknown-linux-gnu
    # aarch64 ILP32 ABI
    aarch64-unknown-linux-gnu_ilp32
    # aarch64 ILP32 ABI big endian
    aarch64_be-unknown-linux-gnu_ilp32
    # aarch64 always support lse
    aarch64-apple-darwin

    # armv7+
    # rustc --print target-list | grep -E '^(arm(eb)?|thumb)(v7|v8|v9)' | grep -E '(-unknown-linux|-none)'
    # armv7-a
    armv7-unknown-linux-gnueabi
    armv7-unknown-linux-gnueabihf
    thumbv7neon-unknown-linux-gnueabihf
    # armv7-r
    armv7r-none-eabi
    # armv7-r big endian
    armebv7r-none-eabi
    # armv7-m
    thumbv7em-none-eabi
    thumbv7em-none-eabihf
    thumbv7m-none-eabi
    # armv8-m
    thumbv8m.base-none-eabi
    thumbv8m.main-none-eabi
    thumbv8m.main-none-eabihf

    # riscv
    # rustc --print target-list | grep -E '^riscv' | grep -E '(-unknown-linux|-none)'
    riscv64gc-unknown-linux-gnu
    # no atomic load/store
    riscv32i-unknown-none-elf
    riscv32im-unknown-none-elf
    riscv32imc-unknown-none-elf
    # riscv32 with atomic
    riscv32imac-unknown-none-elf
)

pre_args=()
if [[ "${1:-}" == "+"* ]]; then
    pre_args+=("$1")
    shift
fi
if [[ $# -gt 0 ]]; then
    targets=("$@")
else
    targets=("${default_targets[@]}")
fi

rustup_target_list=$(rustup ${pre_args[@]+"${pre_args[@]}"} target list)
rustc_target_list=$(rustc ${pre_args[@]+"${pre_args[@]}"} --print target-list)
rustc_version=$(rustc ${pre_args[@]+"${pre_args[@]}"} -Vv | grep 'release: ' | sed 's/release: //')
subcmd=build
if [[ "${rustc_version}" == *"nightly"* ]] || [[ "${rustc_version}" == *"dev"* ]]; then
    rustup ${pre_args[@]+"${pre_args[@]}"} component add rust-src &>/dev/null
    case "${rustc_version}" in
        # -Z check-cfg-features requires 1.61.0-nightly
        1.[0-5]* | 1.60.*) ;;
        *)
            # shellcheck disable=SC2207
            build_script_cfg=($(grep -E 'cargo:rustc-cfg=' build.rs | sed -E 's/^.*cargo:rustc-cfg=//' | sed -E 's/".*$//' | LC_ALL=C sort | uniq))
            check_cfg="-Z unstable-options --check-cfg=names($(IFS=',' && echo "${build_script_cfg[*]}"))"
            echo "base rustflags='${check_cfg}'"
            rustup ${pre_args[@]+"${pre_args[@]}"} component add clippy &>/dev/null
            subcmd=clippy
            ;;
    esac
fi

x() {
    local cmd="$1"
    shift
    (
        set -x
        "${cmd}" "$@"
    )
}
build() {
    local target="$1"
    shift
    args=()
    if ! grep <<<"${rustc_target_list}" -Eq "^${target}$"; then
        echo "target '${target}' not available on ${rustc_version}"
        return 0
    fi
    args+=(${pre_args[@]+"${pre_args[@]}"} hack "${subcmd}")
    if grep <<<"${rustup_target_list}" -Eq "^${target}( |$)"; then
        x rustup ${pre_args[@]+"${pre_args[@]}"} target add "${target}" &>/dev/null
    elif [[ "${rustc_version}" == *"nightly"* ]] || [[ "${rustc_version}" == *"dev"* ]]; then
        case "${target}" in
            *-none* | avr-* | riscv32imc-esp-espidf) args+=(-Z build-std=core) ;;
            *) args+=(-Z build-std) ;;
        esac
    else
        echo "target '${target}' requires nightly compiler"
        return 0
    fi
    if [[ -n "${check_cfg:-}" ]]; then
        args+=(-Z check-cfg-features)
    fi
    args+=(--target "${target}")

    RUSTFLAGS="${RUSTFLAGS:-} ${check_cfg:-}" \
        x cargo "${args[@]}" --feature-powerset --optional-deps --workspace --ignore-private "$@"
    if [[ "${target}" == "aarch64"* ]]; then
        RUSTFLAGS="${RUSTFLAGS:-} ${check_cfg:-} -C target-feature=+lse" \
            x cargo "${args[@]}" --feature-powerset --optional-deps --workspace --ignore-private --target-dir target/lse "$@"
    fi
}

for target in "${targets[@]}"; do
    build "${target}"
done
