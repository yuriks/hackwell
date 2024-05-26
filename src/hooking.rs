use minhook::MinHook;
use std::ffi::c_void;
use std::marker::PhantomData;
use std::mem;
use std::ptr;
use std::sync::atomic::{AtomicPtr, Ordering};

pub(crate) trait UnsafeFnPtr {
    fn as_raw_ptr(&self) -> *mut c_void;
    unsafe fn from_raw_ptr(ptr: *mut c_void) -> Self;
}

macro_rules! impl_UnsafeFnPtr {
    ($($Params:ident),*) => {
        impl<R, $($Params),*> UnsafeFnPtr for unsafe extern "C" fn($($Params),*) -> R {
            fn as_raw_ptr(&self) -> *mut c_void {
                *self as *mut c_void
            }

            unsafe fn from_raw_ptr(ptr: *mut c_void) -> Self {
                mem::transmute(ptr)
            }
        }
    };
}

impl_UnsafeFnPtr!();
impl_UnsafeFnPtr!(T0);
impl_UnsafeFnPtr!(T0, T1);

pub struct Trampoline<FnT> {
    ptr: AtomicPtr<c_void>,
    phantom: PhantomData<FnT>,
}

impl<FnT: UnsafeFnPtr> Trampoline<FnT> {
    pub const fn new() -> Self {
        Trampoline {
            ptr: AtomicPtr::new(ptr::null_mut()),
            phantom: PhantomData,
        }
    }

    pub unsafe fn create_hook(&self, target_fn: *mut c_void, replacement_fn: FnT) {
        self.ptr.store(
            MinHook::create_hook(target_fn, replacement_fn.as_raw_ptr()).unwrap(),
            Ordering::Release,
        );
    }

    pub fn get(&self) -> FnT {
        let trampoline = self.ptr.load(Ordering::Acquire);
        assert!(!trampoline.is_null());
        // SAFETY:
        //   - Transmuting between data and function pointers is allowed by Windows/POSIX platforms.
        //   - A pointer of type T was passed to `create_hook`, which would have been unsound if its
        //     signature/calling convention didn't already match the one of T.
        unsafe { FnT::from_raw_ptr(trampoline) }
    }
}
