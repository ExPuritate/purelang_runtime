use std::{ptr, sync::Arc};

use gc::{Gc, Trace};
use global::{Result, StringTypeReference, ThreadSafe, UnwrapEnum, indexmap, string_name};

#[derive(Clone, Default, Debug, ThreadSafe, global::PartialEq, UnwrapEnum, Trace)]
#[fully_eq]
#[unwrap_enum(ref, ref_mut, try)]
pub enum Value {
    #[default]
    Void,
    True,
    False,
    UInt8(u8),
    UInt16(u16),
    UInt32(u32),
    UInt64(u64),
    UInt128(u128),
    Int8(i8),
    Int16(i16),
    Int32(i32),
    Int64(i64),
    Int128(i128),
    Struct(StructObject),
    #[custom_eq(ptr::eq(a0, a0_))]
    Reference(Gc<ByRefValue>),
    RegisterReference(u64),
}

macro core_type($vm:expr, $name:literal) {
    $vm.get_core_single_type(::global::string_name!($name))
        .unwrap()
}

impl Value {
    pub fn ty(&self, cpu: Arc<CPU>) -> Result<TypeHandle> {
        match self {
            Value::Void => Ok(core_type!(cpu.vm(), "System.Void")),
            Value::True => Ok(core_type!(cpu.vm(), "System.Boolean")),
            Value::False => Ok(core_type!(cpu.vm(), "System.Boolean")),
            Value::UInt8(_) => Ok(core_type!(cpu.vm(), "System.UInt8")),
            Value::UInt16(_) => Ok(core_type!(cpu.vm(), "System.UInt16")),
            Value::UInt32(_) => Ok(core_type!(cpu.vm(), "System.UInt32")),
            Value::UInt64(_) => Ok(core_type!(cpu.vm(), "System.UInt64")),
            Value::UInt128(_) => Ok(core_type!(cpu.vm(), "System.UInt128")),
            Value::Int8(_) => Ok(core_type!(cpu.vm(), "System.Int8")),
            Value::Int16(_) => Ok(core_type!(cpu.vm(), "System.Int16")),
            Value::Int32(_) => Ok(core_type!(cpu.vm(), "System.Int32")),
            Value::Int64(_) => Ok(core_type!(cpu.vm(), "System.Int64")),
            Value::Int128(_) => Ok(core_type!(cpu.vm(), "System.Int128")),
            Value::Struct(s) => Ok(s.ty()),
            Value::Reference(gc) => Ok(gc.ty(cpu.vm())),
            Value::RegisterReference(_) => unreachable!(),
        }
    }
    pub fn string_type_reference(&self) -> StringTypeReference {
        match self {
            Value::Void => StringTypeReference::core_static_single_type("System.Void"),
            Value::True | Value::False => {
                StringTypeReference::core_static_single_type("System.Boolean")
            }
            Value::UInt8(_) => StringTypeReference::core_static_single_type("System.UInt8"),
            Value::UInt16(_) => StringTypeReference::core_static_single_type("System.UInt16"),
            Value::UInt32(_) => StringTypeReference::core_static_single_type("System.UInt32"),
            Value::UInt64(_) => StringTypeReference::core_static_single_type("System.UInt64"),
            Value::UInt128(_) => StringTypeReference::core_static_single_type("System.UInt128"),
            Value::Int8(_) => StringTypeReference::core_static_single_type("System.Int8"),
            Value::Int16(_) => StringTypeReference::core_static_single_type("System.Int16"),
            Value::Int32(_) => StringTypeReference::core_static_single_type("System.Int32"),
            Value::Int64(_) => StringTypeReference::core_static_single_type("System.Int64"),
            Value::Int128(_) => StringTypeReference::core_static_single_type("System.Int128"),
            Value::Struct(s) => s.ty().string_reference(),
            Value::Reference(gc) => gc.string_type_reference(),
            Value::RegisterReference(_) => unreachable!(),
        }
    }
}

#[derive(Clone, Debug, UnwrapEnum, Trace)]
#[unwrap_enum(ref, ref_mut, try)]
pub enum ByRefValue {
    Object(Object),
    Array(Array),
    String(StringValue),
    Null,
}

impl ByRefValue {
    pub fn ty(&self, vm: Arc<VM>) -> TypeHandle {
        match self {
            Self::Object(obj) => obj.ty(),
            Self::Array(arr) => arr.ty(vm),
            Self::String(s) => s.ty(vm),
            Self::Null => core_type!(vm, "System.Null"),
        }
    }
    pub fn string_type_reference(&self) -> StringTypeReference {
        match self {
            Self::Object(obj) => unsafe { &*obj.mt }.ty().string_reference(),
            Self::Array(arr) => StringTypeReference::core_generic_type(
                string_name!("System.Array`1"),
                Arc::new(indexmap! {
                    string_name!("T") => arr.t.string_reference(),
                }),
            ),
            Self::String(_) => StringTypeReference::core_static_single_type("System.String"),
            Self::Null => StringTypeReference::core_static_single_type("System.Null"),
        }
    }
}
mod string_value {
    use std::{
        fmt::{Debug, Display},
        sync::Arc,
    };

    use gc::Trace;
    use global::string_name;

    use crate::{type_system::TypeHandle, vm::VM};

    #[derive(Clone, Trace)]
    pub struct StringValue {
        #[ignore_trace]
        s: String,
    }

    impl StringValue {
        pub fn new(s: String) -> Self {
            Self { s }
        }
        pub fn set(&mut self, val: String) {
            self.s = val;
        }
        pub fn get(&self) -> &str {
            &self.s
        }
    }

    impl StringValue {
        pub fn ty(&self, vm: Arc<VM>) -> TypeHandle {
            vm.get_core_single_type(string_name!("System.String"))
                .unwrap()
        }
    }

    impl Debug for StringValue {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "System.String({})", self.s)
        }
    }

    impl Display for StringValue {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "System.String({})", self.s)
        }
    }

    impl AsRef<str> for StringValue {
        fn as_ref(&self) -> &str {
            self.get()
        }
    }
}

pub use string_value::*;

mod struct_object {
    use crate::value::TypeHandle;
    use std::sync::Arc;

    use super::Value;
    use crate::type_system::get_traits::MTGetParent;
    use crate::type_system::{CommonMethodTable, Struct, StructField};
    use crate::vm::VM;
    use gc::Trace;
    use global::derive_ctor::ctor;
    use global::getset::Getters;
    use global::{IndexMap, Result, StringName, ThreadSafe, errors::RuntimeError};

    #[derive(Clone, derive_more::Debug, ThreadSafe, Trace, ctor)]
    #[ctor(pub(crate) internal_new)]
    pub struct StructObject {
        #[ignore_trace]
        #[debug("{}", unsafe { &**mt }.struct_type().name())]
        pub(crate) mt: *mut CommonMethodTable<Struct>,
        fields: IndexMap<StringName, InstanceField>,
    }

    impl StructObject {
        pub fn make(mt: *mut CommonMethodTable<Struct>) -> Self {
            Self {
                mt,
                fields: get_instance_fields(unsafe { (&*mt).struct_type() }),
            }
        }
        pub fn get_field(&self, k: impl AsRef<str>) -> Result<&InstanceField> {
            self.fields
                .get(k.as_ref())
                .ok_or(RuntimeError::FailedGetField(k.as_ref().into()).into())
        }
        pub fn get_mut_field(&mut self, name: impl AsRef<str>) -> Result<&mut InstanceField> {
            self.fields
                .get_mut(name.as_ref())
                .ok_or(RuntimeError::FailedGetField(name.as_ref().into()).into())
        }
        pub fn ty(&self) -> TypeHandle {
            unsafe { TypeHandle::Struct((*self.mt).struct_type()) }
        }
    }

    impl PartialEq for StructObject {
        fn eq(&self, other: &Self) -> bool {
            (unsafe {
                (&*self.mt)
                    .struct_type()
                    .name()
                    .eq((&*other.mt).struct_type().name())
            }) && self.fields.eq(&other.fields)
        }
    }
    impl Eq for StructObject {}

    #[derive(Clone, Debug, ctor, Getters, Trace)]
    #[ctor(pub(crate) internal_new)]
    #[getset(get = "pub")]
    pub struct InstanceField {
        val: Value,
        #[ignore_trace]
        field: StructField,
    }

    impl PartialEq for InstanceField {
        fn eq(&self, other: &Self) -> bool {
            self.val.eq(&other.val)
        }
    }

    impl Eq for InstanceField {}

    impl InstanceField {
        pub fn set_val(&mut self, v: &Value) {
            self.val = v.clone();
        }
    }

    pub(crate) fn get_instance_fields(t: Arc<Struct>) -> IndexMap<StringName, InstanceField> {
        let mut map = IndexMap::new();
        if let Some(parent) = t.mt()._parent() {
            map.append(&mut get_instance_fields(parent));
        }
        for (k, v) in t.fields() {
            if v.attr().is_static() {
                continue;
            }
            map.insert(
                k.clone(),
                InstanceField {
                    val: Value::Void,
                    field: v.clone(),
                },
            );
        }
        map
    }

    pub(crate) fn get_static_fields(t: Arc<Struct>) -> IndexMap<StringName, InstanceField> {
        let mut map = IndexMap::new();
        if let Some(parent) = t.mt()._parent() {
            map.append(&mut get_static_fields(parent));
        }
        for (k, v) in t.static_fields() {
            map.insert(
                k.clone(),
                InstanceField {
                    val: Value::Void,
                    field: v.clone(),
                },
            );
        }
        map
    }
}

pub use struct_object::{InstanceField as StructInstanceField, StructObject};

#[allow(unused_imports)]
pub(crate) use struct_object::{
    get_instance_fields as struct_get_instance_fields,
    get_static_fields as struct_get_static_fields,
};

mod array {
    use std::{
        fmt,
        ops::{Index, IndexMut},
        ptr,
        slice::SliceIndex,
        sync::Arc,
    };

    use gc::{Gc, Trace};
    use global::{indexmap, string_name};

    use crate::{
        type_system::TypeHandle,
        vm::{CPU, VM},
    };

    use super::{ByRefValue, Value};

    #[derive(Clone, derive_more::Debug, Trace)]
    pub struct Array {
        #[debug("{}", t.name())]
        #[ignore_trace]
        pub(crate) t: TypeHandle,
        inner: Vec<Value>,
    }

    impl Array {
        pub fn alloc(cpu: Arc<CPU>, t: TypeHandle) -> Gc<ByRefValue> {
            cpu.heap_alloc(ByRefValue::Array(Self {
                t,
                inner: Vec::new(),
            }))
        }
        pub fn alloc_with_capacity(
            cpu: Arc<CPU>,
            t: TypeHandle,
            capacity: usize,
        ) -> Gc<ByRefValue> {
            cpu.heap_alloc(ByRefValue::Array(Self {
                t,
                inner: Vec::with_capacity(capacity),
            }))
        }
        pub fn alloc_with_data<T: AsRef<[Value]>>(
            cpu: Arc<CPU>,
            t: TypeHandle,
            data: T,
        ) -> Gc<ByRefValue> {
            cpu.heap_alloc(ByRefValue::Array(Self {
                t,
                inner: data.as_ref().to_vec(),
            }))
        }
        pub fn grow_to(&mut self, len: usize) {
            if self.inner.len() < len {
                self.inner.resize_with(len, Default::default);
            }
        }
    }

    impl<I: SliceIndex<[Value]>> Index<I> for Array {
        type Output = I::Output;

        fn index(&self, index: I) -> &Self::Output {
            &self.inner[index]
        }
    }

    impl<I: SliceIndex<[Value]>> IndexMut<I> for Array {
        fn index_mut(&mut self, index: I) -> &mut Self::Output {
            &mut self.inner[index]
        }
    }

    impl Array {
        pub fn get<I: SliceIndex<[Value]>>(&self, index: I) -> Option<&I::Output> {
            self.inner
                .get(index)
                .map(ptr::from_ref)
                .map(|x| unsafe { &*x })
        }
        pub fn get_mut<I: SliceIndex<[Value]>>(&mut self, index: I) -> Option<&mut I::Output> {
            self.inner
                .get_mut(index)
                .map(ptr::from_mut)
                .map(|x| unsafe { &mut *x })
        }
        pub fn push(&mut self, v: Value) {
            self.inner.push(v);
        }
        pub fn set_values<T: AsRef<[Value]>>(&mut self, values: T) {
            self.inner = values.as_ref().to_vec();
        }
    }

    impl Array {
        pub fn dbg_fmt(&self, vm: Arc<VM>, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.debug_struct(format!("System.Array<{}>", self.t.name()).as_str())
                .field("type", &self.ty(vm).name())
                .field("variables", &self.inner)
                .finish()
        }
    }

    impl Array {
        pub fn ty(&self, vm: Arc<VM>) -> TypeHandle {
            unsafe {
                vm.get_core_single_type(string_name!("System.Array`1"))
                    .unwrap()
                    .make_generic(Arc::new(indexmap! {
                        string_name!("@T") => self.t.clone()
                    }))
                    // it won't fail
                    .unwrap_unchecked()
            }
        }
    }
}

pub use array::*;

mod object {
    use std::sync::Arc;

    use gc::{Gc, Trace};
    use global::{
        IndexMap, Result, StringName, derive_ctor::ctor, errors::RuntimeError, getset::Getters,
    };

    use super::{ByRefValue, Value};
    use crate::type_system::get_traits::MTGetParent;
    use crate::{
        type_system::{Class, ClassField, CommonMethodTable, TypeHandle},
        vm::CPU,
    };

    #[derive(Clone, derive_more::Debug, ctor, Trace)]
    #[ctor(pub(crate) internal_new)]
    pub struct Object {
        #[debug("{}", unsafe { &**mt }.class().name())]
        #[ignore_trace]
        pub(crate) mt: *mut CommonMethodTable<Class>,
        fields: IndexMap<StringName, InstanceField>,
    }

    impl Object {
        pub fn alloc(cpu: Arc<CPU>, mt: *mut CommonMethodTable<Class>) -> Gc<ByRefValue> {
            assert!(!mt.is_null());
            cpu.heap_alloc(ByRefValue::Object(Self {
                mt: mt.cast(),
                fields: get_instance_fields((unsafe { &*mt }).class()),
            }))
        }
        pub fn call_as_this(&self, _cpu: Arc<CPU>, _method_name: StringName) -> Result<Value> {
            todo!()
        }
    }

    impl Object {
        pub fn ty(&self) -> TypeHandle {
            unsafe { TypeHandle::Class((*self.mt).class()) }
        }
    }

    impl Object {
        pub fn get_field(&self, name: StringName) -> Result<&InstanceField> {
            self.fields
                .get(&name)
                .ok_or(RuntimeError::FailedGetField(name).into())
        }
        pub fn get_mut_field(&mut self, name: StringName) -> Result<&mut InstanceField> {
            self.fields
                .get_mut(&name)
                .ok_or(RuntimeError::FailedGetField(name).into())
        }
    }

    #[derive(Clone, Debug, ctor, Getters, Trace)]
    #[ctor(pub(crate) internal_new)]
    #[getset(get = "pub")]
    pub struct InstanceField {
        val: Value,
        #[ignore_trace]
        field: ClassField,
    }

    impl InstanceField {
        pub fn set_val(&mut self, v: &Value) {
            self.val = v.clone();
        }
    }

    pub(crate) fn get_instance_fields(class: Arc<Class>) -> IndexMap<StringName, InstanceField> {
        let mut map = IndexMap::new();
        if let Some(parent) = class.mt()._parent() {
            map.append(&mut get_instance_fields(parent));
        }
        for (k, v) in class.fields() {
            if v.attr().is_static() {
                continue;
            }
            map.insert(
                k.clone(),
                InstanceField {
                    val: Value::Void,
                    field: v.clone(),
                },
            );
        }
        map
    }

    pub(crate) fn get_static_fields(class: Arc<Class>) -> IndexMap<StringName, InstanceField> {
        let mut map = IndexMap::new();
        if let Some(parent) = class.mt()._parent() {
            map.append(&mut get_static_fields(parent));
        }
        for (k, v) in class.static_fields() {
            map.insert(
                k.clone(),
                InstanceField {
                    val: Value::Void,
                    field: v.clone(),
                },
            );
        }
        map
    }
}

pub use object::{InstanceField as ObjectInstanceField, Object};

#[allow(unused_imports)]
pub(crate) use object::{
    get_instance_fields as object_get_instance_fields,
    get_static_fields as object_get_static_fields,
};

use crate::{
    type_system::TypeHandle,
    vm::{CPU, VM},
};

#[cfg(test)]
mod test_satisfactions {}
