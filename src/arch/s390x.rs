// s390x
//
// Refs:
// - z/Architecture Principles of Operation https://publibfp.dhe.ibm.com/epubs/pdf/a227832d.pdf
// - z/Architecture Reference Summary https://www.ibm.com/support/pages/zarchitecture-reference-summary
// - portable-atomic https://github.com/taiki-e/portable-atomic
//
// Generated asm:
// - s390x https://godbolt.org/z/qv8s6o13G
// - s390x (z196) https://godbolt.org/z/jW67E4YEq

#[cfg(not(atomic_maybe_uninit_no_s390x_asm_cc_clobbered))]
#[path = "partword.rs"]
mod partword;

use core::{
    arch::asm,
    mem::{self, MaybeUninit},
    sync::atomic::Ordering,
};

#[cfg(not(atomic_maybe_uninit_no_s390x_asm_cc_clobbered))]
use crate::raw::{AtomicCompareExchange, AtomicSwap};
use crate::raw::{AtomicLoad, AtomicStore};

#[cfg(not(atomic_maybe_uninit_no_s390x_asm_cc_clobbered))]
type XSize = u64;

// Extracts and checks condition code.
#[cfg(not(atomic_maybe_uninit_no_s390x_asm_cc_clobbered))]
#[inline]
fn extract_cc(r: i64) -> bool {
    let r = r.wrapping_add(-268435456) & (1 << 31);
    debug_assert!(r == 0 || r == 2147483648, "r={r}");
    r != 0
}

#[cfg(not(atomic_maybe_uninit_no_s390x_asm_cc_clobbered))]
#[inline]
fn complement(v: u32) -> u32 {
    (v ^ !0).wrapping_add(1)
}

macro_rules! atomic_load_store {
    ($int_type:ident, $l_suffix:tt, $asm_suffix:tt) => {
        impl AtomicLoad for $int_type {
            #[inline]
            unsafe fn atomic_load(
                src: *const MaybeUninit<Self>,
                out: *mut MaybeUninit<Self>,
                _order: Ordering,
            ) {
                debug_assert!(src as usize % mem::size_of::<$int_type>() == 0);
                debug_assert!(out as usize % mem::align_of::<$int_type>() == 0);

                // SAFETY: the caller must uphold the safety contract.
                unsafe {
                    // atomic load is always SeqCst.
                    asm!(
                        // (atomic) load from src to r0
                        concat!("l", $l_suffix, " %r0, 0({src})"),
                        // store r0 to out
                        concat!("st", $asm_suffix, " %r0, 0({out})"),
                        src = in(reg) ptr_reg!(src),
                        out = in(reg) ptr_reg!(out),
                        out("r0") _,
                        options(nostack, preserves_flags),
                    );
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
                        ($fence:tt) => {
                            asm!(
                                // load from val to r0
                                concat!("l", $l_suffix, " %r0, 0({val})"),
                                // (atomic) store r0 to dst
                                concat!("st", $asm_suffix, " %r0, 0({dst})"),
                                $fence,
                                dst = in(reg) ptr_reg!(dst),
                                val = in(reg) ptr_reg!(val),
                                out("r0") _,
                                options(nostack, preserves_flags),
                            )
                        };
                    }
                    match order {
                        // Relaxed and Release stores are equivalent.
                        Ordering::Relaxed | Ordering::Release => atomic_store!(""),
                        // bcr 14,0 (fast-BCR-serialization) requires z196 or later.
                        #[cfg(any(
                            target_feature = "fast-serialization",
                            atomic_maybe_uninit_target_feature = "fast-serialization",
                        ))]
                        Ordering::SeqCst => atomic_store!("bcr 14, 0"),
                        #[cfg(not(any(
                            target_feature = "fast-serialization",
                            atomic_maybe_uninit_target_feature = "fast-serialization",
                        )))]
                        Ordering::SeqCst => atomic_store!("bcr 15, 0"),
                        _ => unreachable!("{:?}", order),
                    }
                }
            }
        }
    };
}

macro_rules! atomic {
    ($int_type:ident, $asm_suffix:tt) => {
        atomic_load_store!($int_type, $asm_suffix, $asm_suffix);
        #[cfg(not(atomic_maybe_uninit_no_s390x_asm_cc_clobbered))]
        impl AtomicSwap for $int_type {
            #[inline]
            unsafe fn atomic_swap(
                dst: *mut MaybeUninit<Self>,
                val: *const MaybeUninit<Self>,
                out: *mut MaybeUninit<Self>,
                _order: Ordering,
            ) {
                debug_assert!(dst as usize % mem::size_of::<$int_type>() == 0);
                debug_assert!(val as usize % mem::align_of::<$int_type>() == 0);
                debug_assert!(out as usize % mem::align_of::<$int_type>() == 0);

                // SAFETY: the caller must uphold the safety contract.
                unsafe {
                    // atomic swap is always SeqCst.
                    asm!(
                        // load from val to val_tmp
                        concat!("l", $asm_suffix, " {val_tmp}, 0({val})"),
                        // (atomic) swap (CAS loop)
                        concat!("l", $asm_suffix, " %r0, 0({dst})"),
                        "2:",
                            concat!("cs", $asm_suffix, " %r0, {val_tmp}, 0({dst})"),
                            "jl 2b",
                        // store r0 to out
                        concat!("st", $asm_suffix, " %r0, 0({out})"),
                        dst = in(reg) ptr_reg!(dst),
                        val = in(reg) ptr_reg!(val),
                        val_tmp = out(reg) _,
                        out = in(reg) ptr_reg!(out),
                        out("r0") _,
                        // Do not use `preserves_flags` because CS modifies the condition code.
                        options(nostack),
                    );
                }
            }
        }
        #[cfg(not(atomic_maybe_uninit_no_s390x_asm_cc_clobbered))]
        impl AtomicCompareExchange for $int_type {
            #[inline]
            unsafe fn atomic_compare_exchange(
                dst: *mut MaybeUninit<Self>,
                old: *const MaybeUninit<Self>,
                new: *const MaybeUninit<Self>,
                out: *mut MaybeUninit<Self>,
                _success: Ordering,
                _failure: Ordering,
            ) -> bool {
                debug_assert!(dst as usize % mem::size_of::<$int_type>() == 0);
                debug_assert!(old as usize % mem::align_of::<$int_type>() == 0);
                debug_assert!(new as usize % mem::align_of::<$int_type>() == 0);
                debug_assert!(out as usize % mem::align_of::<$int_type>() == 0);

                // SAFETY: the caller must uphold the safety contract.
                unsafe {
                    let mut r: i64;
                    // compare_exchange is always SeqCst.
                    asm!(
                        // load from old/new to r0/tmp
                        concat!("l", $asm_suffix, " %r0, 0({old})"),
                        concat!("l", $asm_suffix, " {tmp}, 0({new})"),
                        // (atomic) CAS
                        concat!("cs", $asm_suffix, " %r0, {tmp}, 0({dst})"),
                        // store condition code
                        "ipm {tmp}",
                        // store r0 to out
                        concat!("st", $asm_suffix, " %r0, 0({out})"),
                        dst = in(reg) ptr_reg!(dst),
                        old = in(reg) ptr_reg!(old),
                        new = in(reg) ptr_reg!(new),
                        tmp = out(reg) r,
                        out = in(reg) ptr_reg!(out),
                        out("r0") _,
                        // Do not use `preserves_flags` because CS modifies the condition code.
                        options(nostack),
                    );
                    extract_cc(r)
                }
            }
        }
    };
}

macro_rules! atomic_sub_word {
    ($int_type:ident, $l_suffix:tt, $asm_suffix:tt, $bits:tt, $risbg_swap:tt, $risbg_cas:tt) => {
        atomic_load_store!($int_type, $l_suffix, $asm_suffix);
        #[cfg(not(atomic_maybe_uninit_no_s390x_asm_cc_clobbered))]
        impl AtomicSwap for $int_type {
            #[inline]
            unsafe fn atomic_swap(
                dst: *mut MaybeUninit<Self>,
                val: *const MaybeUninit<Self>,
                out: *mut MaybeUninit<Self>,
                _order: Ordering,
            ) {
                debug_assert!(dst as usize % mem::size_of::<$int_type>() == 0);
                debug_assert!(val as usize % mem::align_of::<$int_type>() == 0);
                debug_assert!(out as usize % mem::align_of::<$int_type>() == 0);
                let (aligned_ptr, shift, _mask) = partword::create_mask_values(dst);

                // SAFETY: the caller must uphold the safety contract.
                unsafe {
                    // Implement sub-word atomic operations using word-sized CAS loop.
                    // Based on assemblies generated by rustc/LLVM.
                    // See also partword.rs.
                    asm!(
                        concat!("l", $l_suffix, " %r0, 0(%r3)"),
                        "l %r3, 0({dst})",
                        "2:",
                            "rll %r14, %r3, 0({shift})",
                            concat!("risbg %r14, %r0, 32, ", $risbg_swap),
                            "rll %r14, %r14, 0({shift_c})",
                            "cs %r3, %r14, 0({dst})",
                            "jl 2b",
                        concat!("rll %r0, %r3, ", $bits ,"({shift})"),
                        concat!("st", $asm_suffix, " %r0, 0({out})"),
                        dst = in(reg) ptr_reg!(aligned_ptr),
                        out = in(reg) ptr_reg!(out),
                        shift = in(reg) shift as u32,
                        shift_c = in(reg) complement(shift as u32),
                        out("r0") _,
                        inout("r3") ptr_reg!(val) => _,
                        out("r14") _,
                        // Do not use `preserves_flags` because CS modifies the condition code.
                        options(nostack),
                    );
                }
            }
        }
        #[cfg(not(atomic_maybe_uninit_no_s390x_asm_cc_clobbered))]
        impl AtomicCompareExchange for $int_type {
            #[inline]
            unsafe fn atomic_compare_exchange(
                dst: *mut MaybeUninit<Self>,
                old: *const MaybeUninit<Self>,
                new: *const MaybeUninit<Self>,
                out: *mut MaybeUninit<Self>,
                _success: Ordering,
                _failure: Ordering,
            ) -> bool {
                debug_assert!(dst as usize % mem::size_of::<$int_type>() == 0);
                debug_assert!(old as usize % mem::align_of::<$int_type>() == 0);
                debug_assert!(new as usize % mem::align_of::<$int_type>() == 0);
                debug_assert!(out as usize % mem::align_of::<$int_type>() == 0);
                let (aligned_ptr, shift, _mask) = partword::create_mask_values(dst);

                // SAFETY: the caller must uphold the safety contract.
                unsafe {
                    let mut r: i64;
                    // Implement sub-word atomic operations using word-sized CAS loop.
                    // Based on assemblies generated by rustc/LLVM.
                    // See also partword.rs.
                    asm!(
                        concat!("ll", $asm_suffix, " %r0, 0(%r3)"),
                        concat!("l", $l_suffix, " %r1, 0(%r4)"),
                        "l %r4, 0({dst})",
                        "2:",
                            concat!("rll %r13, %r4, ", $bits ,"({shift})"),
                            concat!("risbg %r1, %r13, 32, ", $risbg_cas, ", 0"),
                            concat!("ll", $asm_suffix, "r %r13, %r13"),
                            "cr %r13, %r0",
                            "jlh 3f",
                            concat!("rll %r3, %r1, -", $bits ,"({shift_c})"),
                            "cs %r4, %r3, 0({dst})",
                            "jl 2b",
                        "3:",
                        // store condition code
                        "ipm %r0",
                        concat!("st", $asm_suffix, " %r13, 0({out})"),
                        dst = in(reg) ptr_reg!(aligned_ptr),
                        out = in(reg) ptr_reg!(out),
                        shift = in(reg) shift as u32,
                        shift_c = in(reg) complement(shift as u32),
                        out("r0") r,
                        out("r1") _,
                        inout("r3") ptr_reg!(old) => _,
                        inout("r4") ptr_reg!(new) => _,
                        out("r13") _,
                        // Do not use `preserves_flags` because CS modifies the condition code.
                        options(nostack),
                    );
                    extract_cc(r)
                }
            }
        }
    };
}

atomic_sub_word!(i8, "b", "c", "8", "39, 24", "55");
atomic_sub_word!(u8, "b", "c", "8", "39, 24", "55");
atomic_sub_word!(i16, "h", "h", "16", "47, 16", "47");
atomic_sub_word!(u16, "h", "h", "16", "47, 16", "47");
atomic!(i32, "");
atomic!(u32, "");
atomic!(i64, "g");
atomic!(u64, "g");
atomic!(isize, "g");
atomic!(usize, "g");

// https://github.com/llvm/llvm-project/commit/a11f63a952664f700f076fd754476a2b9eb158cc
macro_rules! atomic128 {
    ($int_type:ident) => {
        impl AtomicLoad for $int_type {
            #[inline]
            unsafe fn atomic_load(
                src: *const MaybeUninit<Self>,
                out: *mut MaybeUninit<Self>,
                _order: Ordering,
            ) {
                debug_assert!(src as usize % mem::size_of::<$int_type>() == 0);
                debug_assert!(out as usize % mem::align_of::<$int_type>() == 0);

                // SAFETY: the caller must uphold the safety contract.
                unsafe {
                    // atomic load is always SeqCst.
                    asm!(
                        // (atomic) load from src to out pair
                        "lpq %r0, 0({src})",
                        // store out pair to out
                        "stg %r1, 8({out})",
                        "stg %r0, 0({out})",
                        src = in(reg) ptr_reg!(src),
                        out = in(reg) ptr_reg!(out),
                        // Quadword atomic instructions work with even/odd pair of specified register and subsequent register.
                        out("r0") _, // out (hi)
                        out("r1") _, // out (lo)
                        options(nostack, preserves_flags),
                    );
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
                        ($fence:tt) => {
                            asm!(
                                // load from val to val pair
                                "lg %r1, 8({val})",
                                "lg %r0, 0({val})",
                                // (atomic) store val pair to dst
                                "stpq %r0, 0({dst})",
                                $fence,
                                dst = in(reg) ptr_reg!(dst),
                                val = in(reg) ptr_reg!(val),
                                // Quadword atomic instructions work with even/odd pair of specified register and subsequent register.
                                out("r0") _, // val (hi)
                                out("r1") _, // val (lo)
                                options(nostack, preserves_flags),
                            )
                        };
                    }
                    match order {
                        // Relaxed and Release stores are equivalent.
                        Ordering::Relaxed | Ordering::Release => atomic_store!(""),
                        // bcr 14,0 (fast-BCR-serialization) requires z196 or later.
                        #[cfg(any(
                            target_feature = "fast-serialization",
                            atomic_maybe_uninit_target_feature = "fast-serialization",
                        ))]
                        Ordering::SeqCst => atomic_store!("bcr 14, 0"),
                        #[cfg(not(any(
                            target_feature = "fast-serialization",
                            atomic_maybe_uninit_target_feature = "fast-serialization",
                        )))]
                        Ordering::SeqCst => atomic_store!("bcr 15, 0"),
                        _ => unreachable!("{:?}", order),
                    }
                }
            }
        }
        #[cfg(not(atomic_maybe_uninit_no_s390x_asm_cc_clobbered))]
        impl AtomicSwap for $int_type {
            #[inline]
            unsafe fn atomic_swap(
                dst: *mut MaybeUninit<Self>,
                val: *const MaybeUninit<Self>,
                out: *mut MaybeUninit<Self>,
                _order: Ordering,
            ) {
                debug_assert!(dst as usize % mem::size_of::<$int_type>() == 0);
                debug_assert!(val as usize % mem::align_of::<$int_type>() == 0);
                debug_assert!(out as usize % mem::align_of::<$int_type>() == 0);

                // SAFETY: the caller must uphold the safety contract.
                unsafe {
                    // atomic swap is always SeqCst.
                    asm!(
                        // load from val to val pair
                        "lg %r1, 8({val})",
                        "lg %r0, 0({val})",
                        // (atomic) swap (CAS loop)
                        "lpq %r2, 0({dst})",
                        "2:",
                            "cdsg %r2, %r0, 0({dst})",
                            "jl 2b",
                        // store out pair to out
                        "stg %r3, 8({out})",
                        "stg %r2, 0({out})",
                        dst = inout(reg) ptr_reg!(dst) => _,
                        val = in(reg) ptr_reg!(val),
                        out = inout(reg) ptr_reg!(out) => _,
                        // Quadword atomic instructions work with even/odd pair of specified register and subsequent register.
                        out("r0") _, // val (hi)
                        out("r1") _, // val (lo)
                        lateout("r2") _, // out (hi)
                        lateout("r3") _, // out (lo)
                        // Do not use `preserves_flags` because CDSG modifies the condition code.
                        options(nostack),
                    );
                }
            }
        }
        #[cfg(not(atomic_maybe_uninit_no_s390x_asm_cc_clobbered))]
        impl AtomicCompareExchange for $int_type {
            #[inline]
            unsafe fn atomic_compare_exchange(
                dst: *mut MaybeUninit<Self>,
                old: *const MaybeUninit<Self>,
                new: *const MaybeUninit<Self>,
                out: *mut MaybeUninit<Self>,
                _success: Ordering,
                _failure: Ordering,
            ) -> bool {
                debug_assert!(dst as usize % mem::size_of::<$int_type>() == 0);
                debug_assert!(old as usize % mem::align_of::<$int_type>() == 0);
                debug_assert!(new as usize % mem::align_of::<$int_type>() == 0);
                debug_assert!(out as usize % mem::align_of::<$int_type>() == 0);

                // SAFETY: the caller must uphold the safety contract.
                unsafe {
                    let mut r: i64;
                    // compare_exchange is always SeqCst.
                    asm!(
                        // load from old/new to old/new pairs
                        "lg %r1, 8({old})",
                        "lg %r0, 0({old})",
                        "lg %r13, 8({new})",
                        "lg %r12, 0({new})",
                        // (atomic) CAS
                        "cdsg %r0, %r12, 0({dst})",
                        // store condition code
                        "ipm {r}",
                        // store out pair to out
                        "stg %r1, 8({out})",
                        "stg %r0, 0({out})",
                        dst = in(reg) ptr_reg!(dst),
                        old = in(reg) ptr_reg!(old),
                        new = in(reg) ptr_reg!(new),
                        out = inout(reg) ptr_reg!(out) => _,
                        r = lateout(reg) r,
                        // Quadword atomic instructions work with even/odd pair of specified register and subsequent register.
                        out("r0") _, // old (hi) -> out (hi)
                        out("r1") _, // old (lo) -> out (lo)
                        out("r12") _, // new (hi)
                        out("r13") _, // new (hi)
                        // Do not use `preserves_flags` because CDSG modifies the condition code.
                        options(nostack),
                    );
                    extract_cc(r)
                }
            }
        }
    };
}

atomic128!(i128);
atomic128!(u128);

#[cfg(test)]
mod tests {
    #[cfg(not(atomic_maybe_uninit_no_s390x_asm_cc_clobbered))]
    test_atomic!(isize);
    #[cfg(not(atomic_maybe_uninit_no_s390x_asm_cc_clobbered))]
    test_atomic!(usize);
    #[cfg(not(atomic_maybe_uninit_no_s390x_asm_cc_clobbered))]
    test_atomic!(i8);
    #[cfg(not(atomic_maybe_uninit_no_s390x_asm_cc_clobbered))]
    test_atomic!(u8);
    #[cfg(not(atomic_maybe_uninit_no_s390x_asm_cc_clobbered))]
    test_atomic!(i16);
    #[cfg(not(atomic_maybe_uninit_no_s390x_asm_cc_clobbered))]
    test_atomic!(u16);
    #[cfg(not(atomic_maybe_uninit_no_s390x_asm_cc_clobbered))]
    test_atomic!(i32);
    #[cfg(not(atomic_maybe_uninit_no_s390x_asm_cc_clobbered))]
    test_atomic!(u32);
    #[cfg(not(atomic_maybe_uninit_no_s390x_asm_cc_clobbered))]
    test_atomic!(i64);
    #[cfg(not(atomic_maybe_uninit_no_s390x_asm_cc_clobbered))]
    test_atomic!(u64);
    #[cfg(not(atomic_maybe_uninit_no_s390x_asm_cc_clobbered))]
    test_atomic!(i128);
    #[cfg(not(atomic_maybe_uninit_no_s390x_asm_cc_clobbered))]
    test_atomic!(u128);

    #[cfg(atomic_maybe_uninit_no_s390x_asm_cc_clobbered)]
    test_atomic_load_store!(isize);
    #[cfg(atomic_maybe_uninit_no_s390x_asm_cc_clobbered)]
    test_atomic_load_store!(usize);
    #[cfg(atomic_maybe_uninit_no_s390x_asm_cc_clobbered)]
    test_atomic_load_store!(i8);
    #[cfg(atomic_maybe_uninit_no_s390x_asm_cc_clobbered)]
    test_atomic_load_store!(u8);
    #[cfg(atomic_maybe_uninit_no_s390x_asm_cc_clobbered)]
    test_atomic_load_store!(i16);
    #[cfg(atomic_maybe_uninit_no_s390x_asm_cc_clobbered)]
    test_atomic_load_store!(u16);
    #[cfg(atomic_maybe_uninit_no_s390x_asm_cc_clobbered)]
    test_atomic_load_store!(i32);
    #[cfg(atomic_maybe_uninit_no_s390x_asm_cc_clobbered)]
    test_atomic_load_store!(u32);
    #[cfg(atomic_maybe_uninit_no_s390x_asm_cc_clobbered)]
    test_atomic_load_store!(i64);
    #[cfg(atomic_maybe_uninit_no_s390x_asm_cc_clobbered)]
    test_atomic_load_store!(u64);
    #[cfg(atomic_maybe_uninit_no_s390x_asm_cc_clobbered)]
    test_atomic_load_store!(i128);
    #[cfg(atomic_maybe_uninit_no_s390x_asm_cc_clobbered)]
    test_atomic_load_store!(u128);

    // load/store/swap implementation is not affected by signedness, so it is
    // enough to test only unsigned types.
    #[cfg(not(atomic_maybe_uninit_no_s390x_asm_cc_clobbered))]
    stress_test!(u8);
    #[cfg(not(atomic_maybe_uninit_no_s390x_asm_cc_clobbered))]
    stress_test!(u16);
    #[cfg(not(atomic_maybe_uninit_no_s390x_asm_cc_clobbered))]
    stress_test!(u32);
    #[cfg(not(atomic_maybe_uninit_no_s390x_asm_cc_clobbered))]
    stress_test!(u64);
    #[cfg(not(atomic_maybe_uninit_no_s390x_asm_cc_clobbered))]
    stress_test!(u128);

    #[cfg(atomic_maybe_uninit_no_s390x_asm_cc_clobbered)]
    stress_test_load_store!(u8);
    #[cfg(atomic_maybe_uninit_no_s390x_asm_cc_clobbered)]
    stress_test_load_store!(u16);
    #[cfg(atomic_maybe_uninit_no_s390x_asm_cc_clobbered)]
    stress_test_load_store!(u32);
    #[cfg(atomic_maybe_uninit_no_s390x_asm_cc_clobbered)]
    stress_test_load_store!(u64);
    #[cfg(atomic_maybe_uninit_no_s390x_asm_cc_clobbered)]
    stress_test_load_store!(u128);
}
