use crate::type_system::{
    Assembly, Class, CommonMethod, CommonMethodTable, Struct, TypeHandle, TypeVar,
};
use global::instruction::StringInstruction;
use global::{IndexMap, StringName};
use sealed::sealed;
use std::any::Any;
use std::sync::Arc;

#[sealed]
pub trait GetTypeName {
    fn name(&self) -> StringName;
}

#[sealed]
pub trait GetAssembly {
    fn assembly(&self) -> Option<Arc<Assembly>>;
}

#[sealed]
pub trait GetAssemblyMust {
    fn must_assembly(&self) -> Arc<Assembly>;
}

#[sealed]
pub trait GetInstruction {
    fn instructions(&self) -> Arc<[StringInstruction]>;
}

#[sealed]
impl<T: Any + GetTypeName> GetInstruction for CommonMethod<T> {
    fn instructions(&self) -> Arc<[StringInstruction]> {
        self.instructions.clone()
    }
}

#[sealed]
impl<T: GetAssemblyMust> GetAssembly for T {
    fn assembly(&self) -> Option<Arc<Assembly>> {
        Some(self.must_assembly())
    }
}

#[sealed]
impl GetTypeName for TypeHandle {
    fn name(&self) -> StringName {
        self.name()
    }
}

#[sealed]
impl GetTypeName for Class {
    fn name(&self) -> StringName {
        self.name().clone()
    }
}

#[sealed]
impl GetTypeName for Struct {
    fn name(&self) -> StringName {
        self.name().clone()
    }
}

#[sealed]
impl GetAssembly for TypeHandle {
    fn assembly(&self) -> Option<Arc<Assembly>> {
        self.assembly()
    }
}

#[sealed]
impl GetAssemblyMust for Class {
    fn must_assembly(&self) -> Arc<Assembly> {
        self.assem()
    }
}

#[sealed]
impl GetAssemblyMust for Struct {
    fn must_assembly(&self) -> Arc<Assembly> {
        self.assem()
    }
}

#[sealed]
pub trait GetFieldCount {
    fn field_count(&self) -> usize;
}

#[sealed]
impl GetFieldCount for Class {
    fn field_count(&self) -> usize {
        self.fields().len()
    }
}

#[sealed]
impl GetFieldCount for Struct {
    fn field_count(&self) -> usize {
        self.fields().len()
    }
}

#[sealed]
pub trait GetTypeVars {
    fn type_vars(&self) -> Arc<IndexMap<StringName, TypeVar>>;
}

#[sealed]
impl GetTypeVars for Class {
    fn type_vars(&self) -> Arc<IndexMap<StringName, TypeVar>> {
        self.type_vars.clone()
    }
}

#[sealed]
impl GetTypeVars for Struct {
    fn type_vars(&self) -> Arc<IndexMap<StringName, TypeVar>> {
        self.type_vars.clone()
    }
}

#[sealed]
pub trait GetMethodTable: Any + GetTypeName + Sized {
    fn mt_ptr(&self) -> *mut CommonMethodTable<Self>;
}

#[sealed]
impl GetMethodTable for Class {
    fn mt_ptr(&self) -> *mut CommonMethodTable<Self> {
        self.mt.get()
    }
}

#[sealed]
impl GetMethodTable for Struct {
    fn mt_ptr(&self) -> *mut CommonMethodTable<Self> {
        self.mt.get()
    }
}

#[sealed]
pub trait MTGetParent<T: GetTypeName + Any> {
    fn _parent(&self) -> Option<Arc<T>>;
}

#[sealed]
impl MTGetParent<Struct> for CommonMethodTable<Struct> {
    fn _parent(&self) -> Option<Arc<Struct>> {
        match self.parent.clone()? {
            TypeHandle::Struct(s) => Some(s),
            _ => None,
        }
    }
}

#[sealed]
impl MTGetParent<Class> for CommonMethodTable<Class> {
    fn _parent(&self) -> Option<Arc<Class>> {
        match self.parent.clone()? {
            TypeHandle::Class(c) => Some(c),
            _ => None,
        }
    }
}
