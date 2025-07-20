use std::any::Any;

use global::{IndexMap, StringName};

pub trait Trace: Any {
    fn trace(&self) -> Vec<usize>;
}

impl<T: Trace> Trace for Vec<T> {
    fn trace(&self) -> Vec<usize> {
        let mut result = Vec::new();
        for v in self {
            result.extend(Trace::trace(v));
        }
        result
    }
}

impl<K: Trace, V: Trace> Trace for IndexMap<K, V> {
    fn trace(&self) -> Vec<usize> {
        let mut result = Vec::new();
        for (k, v) in self {
            result.extend(Trace::trace(k));
            result.extend(Trace::trace(v));
        }
        result
    }
}

macro impl_empty($($t:ty)*) {$(
	impl Trace for $t {
		fn trace(&self) -> Vec<usize> { Vec::new() }
	}
)*}

impl_empty! {
    u8
    u16
    u32
    u64
    u128

    i8
    i16
    i32
    i64
    i128

    StringName
}
