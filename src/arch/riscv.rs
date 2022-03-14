// Refs:
// - "Mappings from C/C++ primitives to RISC-V primitives." table in RISC-V Instruction Set Manual:
//   https://five-embeddev.com/riscv-isa-manual/latest/memory.html#sec:memory:porting
// - portable-atomic https://github.com/taiki-e/portable-atomic
//
// Generated asm:
// - riscv64gc https://godbolt.org/z/Ef9so3M8G

use core::{arch::asm, mem::MaybeUninit, sync::atomic::Ordering};

#[cfg(any(target_feature_a, target_feature = "a"))]
use crate::raw::AtomicSwap;
use crate::raw::{AtomicLoad, AtomicStore};

macro_rules! atomic_load_store {
    ($int_type:ident, $asm_suffix:tt) => {
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
                    match order {
                        Ordering::Relaxed => {
                            asm!(
                                // (atomic) load from src to tmp
                                concat!("l", $asm_suffix, " {tmp}, 0({src})"),
                                // store tmp to out
                                concat!("s", $asm_suffix, " {tmp}, 0({out})"),
                                src = in(reg) src,
                                out = in(reg) out,
                                tmp = out(reg) _,
                                options(nostack),
                            );
                        }
                        Ordering::Acquire => {
                            asm!(
                                // (atomic) load from src to tmp
                                concat!("l", $asm_suffix, " {tmp}, 0({src})"),
                                "fence r, rw",
                                // store tmp to out
                                concat!("s", $asm_suffix, " {tmp}, 0({out})"),
                                src = in(reg) src,
                                out = in(reg) out,
                                tmp = out(reg) _,
                                options(nostack),
                            );
                        }
                        Ordering::SeqCst => {
                            asm!(
                                // (atomic) load from src to tmp
                                "fence rw, rw",
                                concat!("l", $asm_suffix, " {tmp}, 0({src})"),
                                "fence r, rw",
                                // store tmp to out
                                concat!("s", $asm_suffix, " {tmp}, 0({out})"),
                                src = in(reg) src,
                                out = in(reg) out,
                                tmp = out(reg) _,
                                options(nostack),
                            );
                        }
                        _ => crate::utils::release_unreachable_unchecked(),
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
                    match order {
                        Ordering::Relaxed => {
                            asm!(
                                // load from val to tmp
                                concat!("l", $asm_suffix, " {tmp}, 0({val})"),
                                // (atomic) store tmp to dst
                                concat!("s", $asm_suffix, " {tmp}, 0({dst})"),
                                dst = in(reg) dst,
                                val = in(reg) val,
                                tmp = out(reg) _,
                                options(nostack),
                            );
                        }
                        // Release and SeqCst stores are equivalent.
                        Ordering::Release | Ordering::SeqCst => {
                            asm!(
                                // load from val to tmp
                                concat!("l", $asm_suffix, " {tmp}, 0({val})"),
                                // (atomic) store tmp to dst
                                "fence rw, w",
                                concat!("s", $asm_suffix, " {tmp}, 0({dst})"),
                                dst = in(reg) dst,
                                val = in(reg) val,
                                tmp = out(reg) _,
                                options(nostack),
                            );
                        }
                        _ => crate::utils::release_unreachable_unchecked(),
                    }
                }
            }
        }
    };
}

macro_rules! atomic {
    ($int_type:ident, $asm_suffix:tt) => {
        atomic_load_store!($int_type, $asm_suffix);
        #[cfg(any(target_feature_a, target_feature = "a"))]
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
                        ($order:tt) => {
                            asm!(
                                // load val to val_tmp
                                concat!("l", $asm_suffix, " {val_tmp}, 0({val})"),
                                // (atomic) swap
                                // - load value from dst and store it to out_tmp
                                // - store value of val_tmp to dst
                                concat!("amoswap.", $asm_suffix, $order, " {out_tmp}, {val_tmp}, 0({dst})"),
                                // store out_tmp to out
                                concat!("s", $asm_suffix, " {out_tmp}, 0({out})"),
                                dst = in(reg) dst,
                                val = in(reg) val,
                                val_tmp = out(reg) _,
                                out = in(reg) out,
                                out_tmp = out(reg) _,
                                options(nostack),
                            )
                        };
                    }
                    match order {
                        Ordering::Relaxed => atomic_swap!(""),
                        Ordering::Acquire => atomic_swap!(".aq"),
                        Ordering::Release => atomic_swap!(".rl"),
                        // AcqRel and SeqCst swaps are equivalent.
                        Ordering::AcqRel | Ordering::SeqCst => atomic_swap!(".aqrl"),
                        _ => crate::utils::release_unreachable_unchecked(),
                    }
                }
            }
        }
    };
}

atomic_load_store!(i8, "b");
atomic_load_store!(u8, "b");
atomic_load_store!(i16, "h");
atomic_load_store!(u16, "h");
atomic!(i32, "w");
atomic!(u32, "w");
#[cfg(target_arch = "riscv64")]
atomic!(i64, "d");
#[cfg(target_arch = "riscv64")]
atomic!(u64, "d");
#[cfg(target_pointer_width = "32")]
atomic!(isize, "w");
#[cfg(target_pointer_width = "32")]
atomic!(usize, "w");
#[cfg(target_pointer_width = "64")]
atomic!(isize, "d");
#[cfg(target_pointer_width = "64")]
atomic!(usize, "d");

#[cfg(test)]
mod tests {
    test_atomic!(isize);
    test_atomic!(usize);
    test_atomic_load_store!(i8);
    test_atomic_load_store!(u8);
    test_atomic_load_store!(i16);
    test_atomic_load_store!(u16);
    test_atomic!(i32);
    test_atomic!(u32);
    #[cfg(target_arch = "riscv64")]
    test_atomic!(i64);
    #[cfg(target_arch = "riscv64")]
    test_atomic!(u64);
}
