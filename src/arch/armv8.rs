// ARMv8 AArch32
//
// LLVM doesn't generate CLREX for ARMv8-M Baseline, but it actually supports CLREX.
// https://developer.arm.com/documentation/dui1095/a/The-Cortex-M23-Instruction-Set/Memory-access-instructions?lang=en
// https://community.arm.com/cfs-file/__key/telligent-evolution-components-attachments/01-2057-00-00-00-01-28-35/Cortex_2D00_M-for-Beginners-_2D00_-2017_5F00_EN_5F00_v2.pdf
//
// Refs:
// - Arm Architecture Reference Manual for A-profile architecture
//   https://developer.arm.com/documentation/ddi0487/latest
// - Armv8-M Architecture Reference Manual
//   https://developer.arm.com/documentation/ddi0553/latest
//
// Generated asm:
// - armv8-a https://godbolt.org/z/Mx8z81463
// - armv8-m baseline https://godbolt.org/z/P51ezojjW
// - armv8-m mainline https://godbolt.org/z/WdajnbYTr

use core::{
    arch::asm,
    mem::{self, MaybeUninit},
    sync::atomic::Ordering,
};

use crate::raw::{AtomicCompareExchange, AtomicLoad, AtomicStore, AtomicSwap};

macro_rules! atomic_rmw {
    ($op:ident, $order:ident) => {
        match $order {
            Ordering::Relaxed => $op!("r", "r"),
            Ordering::Acquire => $op!("a", "r"),
            Ordering::Release => $op!("r", "l"),
            // AcqRel and SeqCst RMWs are equivalent.
            Ordering::AcqRel | Ordering::SeqCst => $op!("a", "l"),
            _ => unreachable!("{:?}", $order),
        }
    };
}

// Adds S suffix if needed. We prefer instruction without S suffix,
// but ARMv8-M Baseline doesn't support thumb2 instructions.
#[cfg(not(any(target_feature = "mclass", atomic_maybe_uninit_target_feature = "mclass")))]
macro_rules! s {
    ($op:tt, $operand:tt) => {
        concat!($op, " ", $operand)
    };
}
#[cfg(any(target_feature = "mclass", atomic_maybe_uninit_target_feature = "mclass"))]
macro_rules! s {
    ($op:tt, $operand:tt) => {
        concat!($op, "s ", $operand)
    };
}

macro_rules! atomic {
    ($int_type:ident, $asm_suffix:tt) => {
        impl AtomicLoad for $int_type {
            #[inline]
            unsafe fn atomic_load(
                src: *const MaybeUninit<Self>,
                out: *mut MaybeUninit<Self>,
                order: Ordering,
            ) {
                debug_assert!(src as usize % mem::size_of::<$int_type>() == 0);
                debug_assert!(out as usize % mem::align_of::<$int_type>() == 0);

                // SAFETY: the caller must uphold the safety contract.
                unsafe {
                    macro_rules! atomic_load {
                        ($acquire:tt) => {
                            asm!(
                                // (atomic) load from src to tmp
                                concat!("ld", $acquire, $asm_suffix, " {tmp}, [{src}]"),
                                // store tmp to out
                                concat!("str", $asm_suffix, " {tmp}, [{out}]"),
                                src = in(reg) src,
                                out = inout(reg) out => _,
                                tmp = lateout(reg) _,
                                options(nostack, preserves_flags),
                            )
                        };
                    }
                    match order {
                        Ordering::Relaxed => atomic_load!("r"),
                        // Acquire and SeqCst loads are equivalent.
                        Ordering::Acquire | Ordering::SeqCst => atomic_load!("a"),
                        _ => unreachable!("{:?}", order),
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
                debug_assert!(dst as usize % mem::size_of::<$int_type>() == 0);
                debug_assert!(val as usize % mem::align_of::<$int_type>() == 0);

                // SAFETY: the caller must uphold the safety contract.
                unsafe {
                    macro_rules! atomic_store {
                        ($release:tt) => {
                            asm!(
                                // load from val to tmp
                                concat!("ldr", $asm_suffix, " {tmp}, [{val}]"),
                                // (atomic) store tmp to dst
                                concat!("st", $release, $asm_suffix, " {tmp}, [{dst}]"),
                                dst = inout(reg) dst => _,
                                val = in(reg) val,
                                tmp = lateout(reg) _,
                                options(nostack, preserves_flags),
                            )
                        };
                    }
                    match order {
                        Ordering::Relaxed => atomic_store!("r"),
                        // Release and SeqCst stores are equivalent.
                        Ordering::Release | Ordering::SeqCst => atomic_store!("l"),
                        _ => unreachable!("{:?}", order),
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
                debug_assert!(dst as usize % mem::size_of::<$int_type>() == 0);
                debug_assert!(val as usize % mem::align_of::<$int_type>() == 0);
                debug_assert!(out as usize % mem::align_of::<$int_type>() == 0);

                // SAFETY: the caller must uphold the safety contract.
                unsafe {
                    macro_rules! swap {
                        ($acquire:tt, $release:tt) => {
                            asm!(
                                // load from val (ptr) to val (val)
                                concat!("ldr", $asm_suffix, " {val}, [{val}]"),
                                // (atomic) swap (LL/SC loop)
                                "2:",
                                    // load from dst to tmp
                                    concat!("ld", $acquire, "ex", $asm_suffix, " {tmp}, [{dst}]"),
                                    // try to store val to dst
                                    concat!("st", $release, "ex", $asm_suffix, " {r}, {val}, [{dst}]"),
                                    // 0 if the store was successful, 1 if no store was performed
                                    "cmp {r}, 0x0",
                                    "bne 2b",
                                // store tmp to out
                                concat!("str", $asm_suffix, " {tmp}, [{out}]"),
                                dst = in(reg) dst,
                                val = inout(reg) val => _,
                                out = in(reg) out,
                                r = out(reg) _,
                                tmp = out(reg) _,
                                // Do not use `preserves_flags` because CMP modifies the condition flags.
                                options(nostack),
                            )
                        };
                    }
                    atomic_rmw!(swap, order);
                }
            }
        }
        impl AtomicCompareExchange for $int_type {
            #[inline]
            unsafe fn atomic_compare_exchange(
                dst: *mut MaybeUninit<Self>,
                old: *const MaybeUninit<Self>,
                new: *const MaybeUninit<Self>,
                out: *mut MaybeUninit<Self>,
                success: Ordering,
                failure: Ordering,
            ) -> bool {
                debug_assert!(dst as usize % mem::size_of::<$int_type>() == 0);
                debug_assert!(old as usize % mem::align_of::<$int_type>() == 0);
                debug_assert!(new as usize % mem::align_of::<$int_type>() == 0);
                debug_assert!(out as usize % mem::align_of::<$int_type>() == 0);
                let order = crate::utils::upgrade_success_ordering(success, failure);

                // SAFETY: the caller must uphold the safety contract.
                unsafe {
                    let mut r: i32;
                    macro_rules! cmpxchg {
                        ($acquire:tt, $release:tt) => {
                            asm!(
                                // load from old/new (ptr) to old/new (val)
                                concat!("ldr", $asm_suffix, " {old}, [{old}]"),
                                concat!("ldr", $asm_suffix, " {new}, [{new}]"),
                                // (atomic) CAS (LL/SC loop)
                                "2:",
                                    // load from dst to tmp
                                    concat!("ld", $acquire, "ex", $asm_suffix, " {tmp}, [{dst}]"),
                                    "cmp {tmp}, {old}",
                                    "bne 3f", // jump if compare failed
                                    // try to store val to dst
                                    concat!("st", $release, "ex", $asm_suffix, " {r}, {new}, [{dst}]"),
                                    // 0 if the store was successful, 1 if no store was performed
                                    "cmp {r}, #0",
                                    "bne 2b", // continue loop if store failed
                                    "b 4f",
                                "3:",
                                    // compare failed, mark r as failed and clear exclusive
                                    "clrex",
                                    s!("mov", "{r}, #1"),
                                "4:",
                                // store tmp to out
                                concat!("str", $asm_suffix, " {tmp}, [{out}]"),
                                dst = in(reg) dst,
                                old = inout(reg) old => _,
                                new = inout(reg) new => _,
                                out = in(reg) out,
                                r = out(reg) r,
                                tmp = out(reg) _,
                                // Do not use `preserves_flags` because CMP and s! modify the condition flags.
                                options(nostack),
                            )
                        };
                    }
                    atomic_rmw!(cmpxchg, order);
                    debug_assert!(r == 0 || r == 1, "r={}", r);
                    // 0 if the store was successful, 1 if no store was performed
                    r == 0
                }
            }
            #[inline]
            unsafe fn atomic_compare_exchange_weak(
                dst: *mut MaybeUninit<Self>,
                old: *const MaybeUninit<Self>,
                new: *const MaybeUninit<Self>,
                out: *mut MaybeUninit<Self>,
                success: Ordering,
                failure: Ordering,
            ) -> bool {
                debug_assert!(dst as usize % mem::size_of::<$int_type>() == 0);
                debug_assert!(old as usize % mem::align_of::<$int_type>() == 0);
                debug_assert!(new as usize % mem::align_of::<$int_type>() == 0);
                debug_assert!(out as usize % mem::align_of::<$int_type>() == 0);
                let order = crate::utils::upgrade_success_ordering(success, failure);

                // SAFETY: the caller must uphold the safety contract.
                unsafe {
                    let mut r: i32;
                    macro_rules! cmpxchg_weak {
                        ($acquire:tt, $release:tt) => {
                            asm!(
                                // load from old/new (ptr) to old/new (val)
                                concat!("ldr", $asm_suffix, " {old}, [{old}]"),
                                concat!("ldr", $asm_suffix, " {new}, [{new}]"),
                                // load from dst to tmp
                                concat!("ld", $acquire, "ex", $asm_suffix, " {tmp}, [{dst}]"),
                                "cmp {tmp}, {old}",
                                "bne 3f",
                                // try to store new to dst
                                concat!("st", $release, "ex", $asm_suffix, " {r}, {new}, [{dst}]"),
                                "b 4f",
                                "3:",
                                    // compare failed, mark r as failed and clear exclusive
                                    "clrex",
                                    s!("mov", "{r}, #1"),
                                "4:",
                                // store tmp to out
                                concat!("str", $asm_suffix, " {tmp}, [{out}]"),
                                dst = in(reg) dst,
                                old = inout(reg) old => _,
                                new = inout(reg) new => _,
                                out = in(reg) out,
                                r = out(reg) r,
                                tmp = out(reg) _,
                                // Do not use `preserves_flags` because CMP and s! modify the condition flags.
                                options(nostack),
                            )
                        };
                    }
                    atomic_rmw!(cmpxchg_weak, order);
                    debug_assert!(r == 0 || r == 1, "r={}", r);
                    // 0 if the store was successful, 1 if no store was performed
                    r == 0
                }
            }
        }
    };
}

atomic!(i8, "b");
atomic!(u8, "b");
atomic!(i16, "h");
atomic!(u16, "h");
atomic!(i32, "");
atomic!(u32, "");
atomic!(isize, "");
atomic!(usize, "");

#[rustfmt::skip]
macro_rules! atomic64 {
    ($int_type:ident) => {
        #[cfg(not(any(target_feature = "mclass", atomic_maybe_uninit_target_feature = "mclass")))]
        impl AtomicLoad for $int_type {
            #[inline]
            unsafe fn atomic_load(
                src: *const MaybeUninit<Self>,
                out: *mut MaybeUninit<Self>,
                order: Ordering,
            ) {
                debug_assert!(src as usize % mem::size_of::<$int_type>() == 0);
                debug_assert!(out as usize % mem::align_of::<$int_type>() == 0);

                // SAFETY: the caller must uphold the safety contract.
                unsafe {
                    macro_rules! atomic_load {
                        ($acquire:tt) => {
                            asm!(
                                // (atomic) load from src to tmp pair
                                concat!("ld", $acquire, "exd r2, r3, [{src}]"),
                                "clrex",
                                // store tmp pair to out
                                "strd r2, r3, [{out}]",
                                src = in(reg) src,
                                out = in(reg) out,
                                // tmp pair - must be even-numbered and not R14
                                out("r2") _,
                                out("r3") _,
                                options(nostack, preserves_flags),
                            )
                        };
                    }
                    match order {
                        Ordering::Relaxed => atomic_load!("r"),
                        // Acquire and SeqCst loads are equivalent.
                        Ordering::Acquire | Ordering::SeqCst => atomic_load!("a"),
                        _ => unreachable!("{:?}", order),
                    }
                }
            }
        }
        #[cfg(not(any(target_feature = "mclass", atomic_maybe_uninit_target_feature = "mclass")))]
        impl AtomicStore for $int_type {
            #[inline]
            unsafe fn atomic_store(
                dst: *mut MaybeUninit<Self>,
                val: *const MaybeUninit<Self>,
                order: Ordering,
            ) {
                debug_assert!(dst as usize % mem::size_of::<$int_type>() == 0);
                debug_assert!(val as usize % mem::align_of::<$int_type>() == 0);

                // SAFETY: the caller must uphold the safety contract.
                unsafe {
                    macro_rules! store {
                        ($acquire:tt, $release:tt) => {
                            asm!(
                                // load from val to val pair
                                "ldrd r2, r3, [{val}]",
                                // (atomic) store val pair to dst (LL/SC loop)
                                "2:",
                                    // load from dst to tmp pair
                                    concat!("ld", $acquire, "exd r4, r5, [{dst}]"),
                                    // try to store val pair to dst
                                    concat!("st", $release, "exd {r}, r2, r3, [{dst}]"),
                                    // 0 if the store was successful, 1 if no store was performed
                                    "cmp {r}, 0x0",
                                    "bne 2b",
                                dst = inout(reg) dst => _,
                                val = in(reg) val,
                                r = lateout(reg) _,
                                // val pair - must be even-numbered and not R14
                                out("r2") _,
                                out("r3") _,
                                // tmp pair - must be even-numbered and not R14
                                out("r4") _,
                                out("r5") _,
                                // Do not use `preserves_flags` because CMP modifies the condition flags.
                                options(nostack),
                            )
                        };
                    }
                    atomic_rmw!(store, order);
                }
            }
        }
        #[cfg(not(any(target_feature = "mclass", atomic_maybe_uninit_target_feature = "mclass")))]
        impl AtomicSwap for $int_type {
            #[inline]
            unsafe fn atomic_swap(
                dst: *mut MaybeUninit<Self>,
                val: *const MaybeUninit<Self>,
                out: *mut MaybeUninit<Self>,
                order: Ordering,
            ) {
                debug_assert!(dst as usize % mem::size_of::<$int_type>() == 0);
                debug_assert!(val as usize % mem::align_of::<$int_type>() == 0);
                debug_assert!(out as usize % mem::align_of::<$int_type>() == 0);

                // SAFETY: the caller must uphold the safety contract.
                unsafe {
                    macro_rules! swap {
                        ($acquire:tt, $release:tt) => {
                            asm!(
                                // load from val to val pair
                                "ldrd r2, r3, [{val}]",
                                // (atomic) swap (LL/SC loop)
                                "2:",
                                    // load from dst to out pair
                                    concat!("ld", $acquire, "exd r4, r5, [{dst}]"),
                                    // try to store val pair to dst
                                    concat!("st", $release, "exd {r}, r2, r3, [{dst}]"),
                                    // 0 if the store was successful, 1 if no store was performed
                                    "cmp {r}, 0x0",
                                    "bne 2b",
                                // store out pair to out
                                "strd r4, r5, [{out}]",
                                dst = inout(reg) dst => _,
                                val = in(reg) val,
                                out = inout(reg) out => _,
                                r = lateout(reg) _,
                                // val pair - must be even-numbered and not R14
                                out("r2") _,
                                out("r3") _,
                                // out pair - must be even-numbered and not R14
                                out("r4") _,
                                out("r5") _,
                                // Do not use `preserves_flags` because CMP modifies the condition flags.
                                options(nostack),
                            )
                        };
                    }
                    atomic_rmw!(swap, order);
                }
            }
        }
        #[cfg(not(any(target_feature = "mclass", atomic_maybe_uninit_target_feature = "mclass")))]
        impl AtomicCompareExchange for $int_type {
            #[inline]
            unsafe fn atomic_compare_exchange(
                dst: *mut MaybeUninit<Self>,
                old: *const MaybeUninit<Self>,
                new: *const MaybeUninit<Self>,
                out: *mut MaybeUninit<Self>,
                success: Ordering,
                failure: Ordering,
            ) -> bool {
                debug_assert!(dst as usize % mem::size_of::<$int_type>() == 0);
                debug_assert!(old as usize % mem::align_of::<$int_type>() == 0);
                debug_assert!(new as usize % mem::align_of::<$int_type>() == 0);
                debug_assert!(out as usize % mem::align_of::<$int_type>() == 0);
                let order = crate::utils::upgrade_success_ordering(success, failure);

                // SAFETY: the caller must uphold the safety contract.
                unsafe {
                    let mut r: i32;
                    macro_rules! cmpxchg {
                        ($acquire:tt, $release:tt) => {
                            asm!(
                                "ldrd r2, r3, [{old}]",
                                "ldrd r8, r9, [{new}]",
                                // (atomic) CAS (LL/SC loop)
                                "2:",
                                    concat!("ld", $acquire, "exd r4, r5, [{dst}]"),
                                    "eor {tmp}, r5, r3",
                                    "eor {r}, r4, r2",
                                    "orrs {r}, {r}, {tmp}",
                                    "bne 3f", // jump if compare failed
                                    concat!("st", $release, "exd  {r}, r8, r9, [{dst}]"),
                                    // 0 if the store was successful, 1 if no store was performed
                                    "cmp {r}, #0",
                                    "bne 2b", // continue loop if store failed
                                    "b 4f",
                                "3:",
                                    // compare failed, mark r as failed and clear exclusive
                                    "clrex",
                                    s!("mov", "{r}, #1"),
                                "4:",
                                // store out pair to out
                                "strd r4, r5, [{out}]",
                                dst = inout(reg) dst => _,
                                r = lateout(reg) r,
                                old = in(reg) old,
                                new = in(reg) new,
                                out = inout(reg) out => _,
                                tmp = out(reg) _,
                                // old pair - must be even-numbered and not R14
                                out("r2") _,
                                out("r3") _,
                                // out pair - must be even-numbered and not R14
                                out("r4") _,
                                out("r5") _,
                                // new pair - must be even-numbered and not R14
                                out("r8") _,
                                out("r9") _,
                                // Do not use `preserves_flags` because CMP, ORRS, and s! modify the condition flags.
                                options(nostack),
                            )
                        };
                    }
                    atomic_rmw!(cmpxchg, order);
                    debug_assert!(r == 0 || r == 1, "r={}", r);
                    // 0 if the store was successful, 1 if no store was performed
                    r == 0
                }
            }
            #[inline]
            unsafe fn atomic_compare_exchange_weak(
                dst: *mut MaybeUninit<Self>,
                old: *const MaybeUninit<Self>,
                new: *const MaybeUninit<Self>,
                out: *mut MaybeUninit<Self>,
                success: Ordering,
                failure: Ordering,
            ) -> bool {
                debug_assert!(dst as usize % mem::size_of::<$int_type>() == 0);
                debug_assert!(old as usize % mem::align_of::<$int_type>() == 0);
                debug_assert!(new as usize % mem::align_of::<$int_type>() == 0);
                debug_assert!(out as usize % mem::align_of::<$int_type>() == 0);
                let order = crate::utils::upgrade_success_ordering(success, failure);

                // SAFETY: the caller must uphold the safety contract.
                unsafe {
                    let mut r: i32;
                    macro_rules! cmpxchg_weak {
                        ($acquire:tt, $release:tt) => {
                            asm!(
                                "ldrd r2, r3, [{old}]",
                                "ldrd r8, r9, [{new}]",
                                concat!("ld", $acquire, "exd r4, r5, [{dst}]"),
                                "eor {tmp}, r5, r3",
                                "eor {r}, r4, r2",
                                "orrs {r}, {r}, {tmp}",
                                "bne 3f", // jump if compare failed
                                concat!("st", $release, "exd  {r}, r8, r9, [{dst}]"),
                                "b 4f",
                                "3:",
                                    // compare failed, mark r as failed and clear exclusive
                                    "clrex",
                                    s!("mov", "{r}, #1"),
                                "4:",
                                // store out pair to out
                                "strd r4, r5, [{out}]",
                                dst = inout(reg) dst => _,
                                r = lateout(reg) r,
                                old = in(reg) old,
                                new = in(reg) new,
                                out = inout(reg) out => _,
                                tmp = out(reg) _,
                                // old pair - must be even-numbered and not R14
                                out("r2") _,
                                out("r3") _,
                                // out pair - must be even-numbered and not R14
                                out("r4") _,
                                out("r5") _,
                                // new pair - must be even-numbered and not R14
                                out("r8") _,
                                out("r9") _,
                                // Do not use `preserves_flags` because ORRS and s! modify the condition flags.
                                options(nostack),
                            )
                        };
                    }
                    atomic_rmw!(cmpxchg_weak, order);
                    debug_assert!(r == 0 || r == 1, "r={}", r);
                    // 0 if the store was successful, 1 if no store was performed
                    r == 0
                }
            }
        }
    };
}

atomic64!(i64);
atomic64!(u64);

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

    // load/store/swap implementation is not affected by signedness, so it is
    // enough to test only unsigned types.
    stress_test!(u8);
    stress_test!(u16);
    stress_test!(u32);
    stress_test!(u64);
}
