#![allow(static_mut_refs)]
#![feature(downcast_unchecked)]
#![feature(decl_macro)]

pub use derives::*;

mod trace;

use std::{
    any::Any,
    collections::HashSet,
    fmt::Debug,
    marker::PhantomData,
    mem::MaybeUninit,
    ops::{Deref, DerefMut},
    ptr,
    sync::Once,
};

pub use trace::Trace;

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Gc<T> {
    inner: InnerGc,
    _phantom_data: PhantomData<*mut T>,
}

impl<T: Trace> Gc<T> {
    /// Create a root reference to `val`
    pub fn new(val: T) -> Self {
        let val = Box::leak(Box::new(val));
        let inner = InnerGc {
            data: val as *mut T,
        };
        let this = Self {
            inner,
            _phantom_data: PhantomData,
        };
        all().push(inner);
        roots().push(inner);
        this
    }
    pub fn root(&self) {
        if !roots().contains(&self.inner) {
            roots().push(self.inner)
        }
    }
    pub fn unroot(&self) {
        roots().retain(|x| self.inner.eq(x));
    }
}

impl<T: Trace> Trace for Gc<T> {
    fn trace(&self) -> Vec<usize> {
        (**self).trace()
    }
}

impl<T: Trace> Deref for Gc<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        unsafe {
            let x: &dyn Any = &*self.inner.data;
            x.downcast_ref_unchecked()
        }
    }
}

impl<T: Trace> DerefMut for Gc<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe {
            let x: &mut dyn Any = &mut *self.inner.data;
            x.downcast_mut_unchecked()
        }
    }
}

impl<T: Trace + Debug> std::fmt::Pointer for Gc<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:p}", self.inner.data)
    }
}

impl<T: Trace + Debug> Debug for Gc<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Pointer::fmt(self, f)
    }
}

#[derive(Clone, Copy, Eq)]
struct InnerGc {
    data: *mut dyn Trace,
}

impl PartialEq for InnerGc {
    fn eq(&self, other: &Self) -> bool {
        ptr::addr_eq(self.data, other.data)
    }
}

fn roots() -> &'static mut Vec<InnerGc> {
    static mut DATA: MaybeUninit<Vec<InnerGc>> = MaybeUninit::uninit();
    static ONCE: Once = Once::new();

    unsafe {
        ONCE.call_once(|| {
            DATA = MaybeUninit::new(Vec::new());
        });
        DATA.assume_init_mut()
    }
}

fn all() -> &'static mut Vec<InnerGc> {
    static mut DATA: MaybeUninit<Vec<InnerGc>> = MaybeUninit::uninit();
    static ONCE: Once = Once::new();

    unsafe {
        ONCE.call_once(|| {
            DATA = MaybeUninit::new(Vec::new());
        });
        DATA.assume_init_mut()
    }
}

pub fn collect() {
    let mut marked_set = HashSet::new();
    for gc in roots() {
        unsafe {
            for ele in (*gc.data).trace() {
                marked_set.insert(ele);
            }
        }
    }
    for (i, gc) in all().iter().enumerate() {
        if !marked_set.contains(&i) {
            unsafe {
                gc.data.drop_in_place();
            }
        }
    }
}
