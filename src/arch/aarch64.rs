// Refs:
// - ARM Compiler armasm User Guide
//   https://developer.arm.com/documentation/dui0801/latest
// - Arm Architecture Reference Manual for A-profile architecture
//   https://developer.arm.com/documentation/ddi0487/latest
//
// Generated asm:
// - aarch64 https://godbolt.org/z/68n9h793K

use core::{arch::asm, mem::MaybeUninit, sync::atomic::Ordering};

use crate::raw::{AtomicLoad, AtomicStore, AtomicSwap};

macro_rules! atomic {
    ($int_type:ident, $asm_suffix:tt, $asm_modifier:tt) => {
        impl AtomicLoad for $int_type {
            #[inline]
            unsafe fn atomic_load(
                src: *const MaybeUninit<Self>,
                out: *mut MaybeUninit<Self>,
                order: Ordering,
            ) {
                // clippy bug that does not recognize safety comments inside macros.
                #[allow(clippy::undocumented_unsafe_blocks)]
                // SAFETY: the caller must uphold the safety contract for `atomic_load`.
                unsafe {
                    macro_rules! atomic_load {
                        ($acq:tt) => {
                            asm!(
                                // (atomic) load from src to tmp
                                concat!("ld", $acq, "r", $asm_suffix, " {tmp", $asm_modifier, "}, [{src}]"),
                                // store tmp to out
                                concat!("str", $asm_suffix, " {tmp", $asm_modifier, "}, [{out}]"),
                                src = in(reg) src,
                                out = in(reg) out,
                                tmp = out(reg) _,
                                options(nostack),
                            )
                        };
                    }
                    match order {
                        Ordering::Relaxed => atomic_load!(""),
                        // Acquire and SeqCst loads are equivalent.
                        Ordering::Acquire | Ordering::SeqCst => atomic_load!("a"),
                        _ => crate::utils::ordering_unreachable_unchecked(order),
                    }
                }
            }
        }
        impl AtomicStore for $int_type {
            #[inline]
            unsafe fn atomic_store(
                dst: *mut MaybeUninit<Self>,
                val: *const MaybeUninit<Self>,
                order: Ordering,
            ) {
                // clippy bug that does not recognize safety comments inside macros.
                #[allow(clippy::undocumented_unsafe_blocks)]
                // SAFETY: the caller must uphold the safety contract for `atomic_store`.
                unsafe {
                    macro_rules! atomic_store {
                        ($rel:tt) => {
                            asm!(
                                // load from val to tmp
                                concat!("ldr", $asm_suffix, " {tmp", $asm_modifier, "}, [{val}]"),
                                // (atomic) store tmp to dst
                                concat!("st", $rel, "r", $asm_suffix, " {tmp", $asm_modifier, "}, [{dst}]"),
                                dst = in(reg) dst,
                                val = in(reg) val,
                                tmp = out(reg) _,
                                options(nostack),
                            )
                        };
                    }
                    match order {
                        Ordering::Relaxed => atomic_store!(""),
                        // Release and SeqCst stores are equivalent.
                        Ordering::Release | Ordering::SeqCst => atomic_store!("l"),
                        _ => crate::utils::ordering_unreachable_unchecked(order),
                    }
                }
            }
        }
        impl AtomicSwap for $int_type {
            #[inline]
            unsafe fn atomic_swap(
                dst: *mut MaybeUninit<Self>,
                val: *const MaybeUninit<Self>,
                out: *mut MaybeUninit<Self>,
                order: Ordering,
            ) {
                // clippy bug that does not recognize safety comments inside macros.
                #[allow(clippy::undocumented_unsafe_blocks)]
                // SAFETY: the caller must uphold the safety contract for `atomic_swap`.
                unsafe {
                    macro_rules! atomic_swap {
                        ($acq:tt, $rel:tt) => {
                            asm!(
                                // load from val to val_tmp
                                concat!("ldr", $asm_suffix, " {val_tmp", $asm_modifier, "}, [{val}]"),
                                // (atomic) swap
                                "2:",
                                    // load from dst to out_tmp
                                    concat!("ld", $acq, "xr", $asm_suffix, " {out_tmp", $asm_modifier, "}, [{dst}]"),
                                    // store val to dst
                                    concat!("st", $rel, "xr", $asm_suffix, " {r:w}, {val_tmp", $asm_modifier, "}, [{dst}]"),
                                    // 0 if the store was successful, 1 if no store was performed
                                    "cbnz {r:w}, 2b",
                                // store out_tmp to out
                                concat!("str", $asm_suffix, " {out_tmp", $asm_modifier, "}, [{out}]"),
                                dst = in(reg) dst,
                                val = in(reg) val,
                                val_tmp = out(reg) _,
                                out = in(reg) out,
                                out_tmp = out(reg) _,
                                r = out(reg) _,
                                options(nostack),
                            )
                        };
                    }
                    match order {
                        Ordering::Relaxed => atomic_swap!("", ""),
                        Ordering::Acquire => atomic_swap!("a", ""),
                        Ordering::Release => atomic_swap!("", "l"),
                        // AcqRel and SeqCst swaps are equivalent.
                        Ordering::AcqRel | Ordering::SeqCst => atomic_swap!("a", "l"),
                        _ => crate::utils::ordering_unreachable_unchecked(order),
                    }
                }
            }
        }
    };
}

atomic!(i8, "b", ":w");
atomic!(u8, "b", ":w");
atomic!(i16, "h", ":w");
atomic!(u16, "h", ":w");
atomic!(i32, "", ":w");
atomic!(u32, "", ":w");
atomic!(i64, "", "");
atomic!(u64, "", "");
#[cfg(target_pointer_width = "32")]
atomic!(isize, "", ":w");
#[cfg(target_pointer_width = "32")]
atomic!(usize, "", ":w");
#[cfg(target_pointer_width = "64")]
atomic!(isize, "", "");
#[cfg(target_pointer_width = "64")]
atomic!(usize, "", "");

macro_rules! atomic128 {
    ($int_type:ident) => {
        impl AtomicLoad for $int_type {
            #[inline]
            unsafe fn atomic_load(
                src: *const MaybeUninit<Self>,
                out: *mut MaybeUninit<Self>,
                order: Ordering,
            ) {
                // clippy bug that does not recognize safety comments inside macros.
                #[allow(clippy::undocumented_unsafe_blocks)]
                // SAFETY: the caller must uphold the safety contract for `atomic_load`.
                unsafe {
                    macro_rules! atomic_load {
                        ($acq:tt, $rel:tt) => {
                            asm!(
                                // (atomic) load from src to tmp pair
                                "2:",
                                    // load from src to tmp pair
                                    concat!("ld", $acq, "xp {tmp_lo}, {tmp_hi}, [{src}]"),
                                    // store tmp pair to src
                                    concat!("st", $rel, "xp {r:w}, {tmp_lo}, {tmp_hi}, [{src}]"),
                                    // 0 if the store was successful, 1 if no store was performed
                                    "cbnz {r:w}, 2b",
                                // store tmp pair to out
                                "stp {tmp_lo}, {tmp_hi}, [{out}]",
                                src = in(reg) src,
                                out = in(reg) out,
                                tmp_hi = out(reg) _,
                                tmp_lo = out(reg) _,
                                r = out(reg) _,
                                options(nostack),
                            )
                        };
                    }
                    match order {
                        Ordering::Relaxed => atomic_load!("", ""),
                        Ordering::Acquire => atomic_load!("a", ""),
                        Ordering::SeqCst => atomic_load!("a", "l"),
                        _ => crate::utils::ordering_unreachable_unchecked(order),
                    }
                }
            }
        }
        impl AtomicStore for $int_type {
            #[inline]
            unsafe fn atomic_store(
                dst: *mut MaybeUninit<Self>,
                val: *const MaybeUninit<Self>,
                order: Ordering,
            ) {
                // clippy bug that does not recognize safety comments inside macros.
                #[allow(clippy::undocumented_unsafe_blocks)]
                // SAFETY: the caller must uphold the safety contract for `atomic_store`.
                unsafe {
                    macro_rules! atomic_store {
                        ($acq:tt, $rel:tt) => {
                            asm!(
                                // load from val to val pair
                                "ldp {val_lo}, {val_hi}, [{val}]",
                                // (atomic) store val pair to dst
                                "2:",
                                    // load from dst to tmp pair
                                    concat!("ld", $acq, "xp {tmp_lo}, {tmp_hi}, [{dst}]"),
                                    // store val pair to dst
                                    concat!("st", $rel, "xp {r:w}, {val_lo}, {val_hi}, [{dst}]"),
                                    // 0 if the store was successful, 1 if no store was performed
                                    "cbnz {r:w}, 2b",
                                dst = in(reg) dst,
                                val = in(reg) val,
                                val_hi = out(reg) _,
                                val_lo = out(reg) _,
                                tmp_hi = out(reg) _,
                                tmp_lo = out(reg) _,
                                r = out(reg) _,
                                options(nostack),
                            )
                        };
                    }
                    match order {
                        Ordering::Relaxed => atomic_store!("", ""),
                        Ordering::Release => atomic_store!("", "l"),
                        Ordering::SeqCst => atomic_store!("a", "l"),
                        _ => crate::utils::ordering_unreachable_unchecked(order),
                    }
                }
            }
        }
        impl AtomicSwap for $int_type {
            #[inline]
            unsafe fn atomic_swap(
                dst: *mut MaybeUninit<Self>,
                val: *const MaybeUninit<Self>,
                out: *mut MaybeUninit<Self>,
                order: Ordering,
            ) {
                // clippy bug that does not recognize safety comments inside macros.
                #[allow(clippy::undocumented_unsafe_blocks)]
                // SAFETY: the caller must uphold the safety contract for `atomic_swap`.
                unsafe {
                    macro_rules! atomic_swap {
                        ($acq:tt, $rel:tt) => {
                            asm!(
                                // load from val to val pair
                                "ldp {val_lo}, {val_hi}, [{val}]",
                                // (atomic) swap
                                "2:",
                                    // load from dst to tmp pair
                                    concat!("ld", $acq, "xp {tmp_lo}, {tmp_hi}, [{dst}]"),
                                    // store val pair to dst
                                    concat!("st", $rel, "xp {r:w}, {val_lo}, {val_hi}, [{dst}]"),
                                    // 0 if the store was successful, 1 if no store was performed
                                    "cbnz {r:w}, 2b",
                                // store tmp pair to out
                                "stp {tmp_lo}, {tmp_hi}, [{out}]",
                                dst = in(reg) dst,
                                val = in(reg) val,
                                out = in(reg) out,
                                val_hi = out(reg) _,
                                val_lo = out(reg) _,
                                tmp_hi = out(reg) _,
                                tmp_lo = out(reg) _,
                                r = out(reg) _,
                                options(nostack),
                            )
                        };
                    }
                    match order {
                        Ordering::Relaxed => atomic_swap!("", ""),
                        Ordering::Acquire => atomic_swap!("a", ""),
                        Ordering::Release => atomic_swap!("", "l"),
                        // AcqRel and SeqCst swaps are equivalent.
                        Ordering::AcqRel | Ordering::SeqCst => atomic_swap!("a", "l"),
                        _ => crate::utils::ordering_unreachable_unchecked(order),
                    }
                }
            }
        }
    };
}

atomic128!(i128);
atomic128!(u128);

#[cfg(test)]
mod tests {
    test_atomic!(isize);
    test_atomic!(usize);
    test_atomic!(i8);
    test_atomic!(u8);
    test_atomic!(i16);
    test_atomic!(u16);
    test_atomic!(i32);
    test_atomic!(u32);
    test_atomic!(i64);
    test_atomic!(u64);
    test_atomic!(i128);
    test_atomic!(u128);
}
