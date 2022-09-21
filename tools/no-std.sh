#!/bin/bash
set -euo pipefail
IFS=$'\n\t'
cd "$(dirname "$0")"/..

trap -- 'exit 0' SIGINT

default_targets=(
    # armv6-m
    thumbv6m-none-eabi
    # armv7-m
    thumbv7em-none-eabi
    thumbv7em-none-eabihf
    thumbv7m-none-eabi
    # armv8-m
    thumbv8m.base-none-eabi
    thumbv8m.main-none-eabi
    thumbv8m.main-none-eabihf
)

x() {
    local cmd="$1"
    shift
    (
        set -x
        "${cmd}" "$@"
    )
}
bail() {
    echo "error: $*" >&2
    exit 1
}

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
nightly=''
if [[ "${rustc_version}" == *"nightly"* ]] || [[ "${rustc_version}" == *"dev"* ]]; then
    nightly=1
    rustup ${pre_args[@]+"${pre_args[@]}"} component add rust-src &>/dev/null
fi

run() {
    local target="$1"
    shift
    local args=(${pre_args[@]+"${pre_args[@]}"})
    local target_rustflags="${RUSTFLAGS:-}"
    if ! grep <<<"${rustc_target_list}" -Eq "^${target}$"; then
        if [[ ! -f "target-specs/${target}.json" ]]; then
            echo "target '${target}' not available on ${rustc_version} (skipped)"
            return 0
        fi
        target_flags=(--target "$(pwd)/target-specs/${target}.json")
    else
        target_flags=(--target "${target}")
    fi
    args+=(run "${target_flags[@]}")
    if grep <<<"${rustup_target_list}" -Eq "^${target}( |$)"; then
        x rustup ${pre_args[@]+"${pre_args[@]}"} target add "${target}" &>/dev/null
    elif [[ -n "${nightly}" ]]; then
        args+=(-Z build-std="core")
    else
        echo "target '${target}' requires nightly compiler (skipped)"
        return 0
    fi

    case "${target}" in
        thumb*)
            (
                cd tests/cortex-m
                RUSTFLAGS="${target_rustflags} -C link-arg=-Tlink.x" \
                    x cargo "${args[@]}" "$@"
                RUSTFLAGS="${target_rustflags} -C link-arg=-Tlink.x" \
                    x cargo "${args[@]}" --release "$@"
            )
            ;;
        *) bail "unrecognized target '${target}'" ;;
    esac
}

for target in "${targets[@]}"; do
    run "${target}"
done
