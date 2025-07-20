use std::sync::Arc;

use derive_more::{TryUnwrap, Unwrap};
use global::{
    IndexMap, Result, StringName, StringTypeReference, ThreadSafe, WithType, errors::RuntimeError,
};

use super::{Assembly, Class, Interface, Struct, TypeVar};

#[derive(Clone, Unwrap, TryUnwrap, ThreadSafe, WithType)]
#[unwrap(ref, ref_mut)]
#[try_unwrap(ref, ref_mut)]
#[with_type(repr = u8)]
#[with_type(derive = (Debug, Clone, Copy, PartialEq, Eq))]
pub enum TypeHandle {
    Class(Arc<Class>),
    Interface(Arc<Interface>),
    Struct(Arc<Struct>),
    Generic(StringName),
    Unloaded(StringTypeReference),
}

impl TypeHandle {
    pub fn name(&self) -> StringName {
        match self {
            Self::Class(class) => class.name().clone(),
            Self::Interface(interface) => interface.name().clone(),
            Self::Struct(s) => s.name().clone(),
            Self::Generic(string_name) => string_name.clone(),
            Self::Unloaded(r) => r.string_name_repr(),
        }
    }
    pub fn assembly(&self) -> Option<Arc<Assembly>> {
        match self {
            TypeHandle::Class(class) => Some(class.assem()),
            TypeHandle::Interface(interface) => Some(interface.assem()),
            Self::Struct(s) => Some(s.assem()),
            TypeHandle::Generic(_) => None,
            Self::Unloaded(_) => None,
        }
    }
    pub fn string_reference(&self) -> StringTypeReference {
        match self {
            TypeHandle::Class(class) => class.string_reference(),
            TypeHandle::Interface(interface) => interface.string_reference(),
            TypeHandle::Struct(s) => s.string_reference(),
            TypeHandle::Generic(string_name) => StringTypeReference::Generic(string_name.clone()),
            Self::Unloaded(r) => r.clone(),
        }
    }
}

impl TypeHandle {
    pub fn make_generic(&self, type_vars: Arc<IndexMap<StringName, TypeHandle>>) -> Result<Self> {
        let this = match self {
            TypeHandle::Class(class) => class
                .clone()
                .make_generic(type_vars)
                .map(TypeHandle::Class)?,
            TypeHandle::Interface(_interface) => todo!(),
            TypeHandle::Struct(s) => s.clone().make_generic(type_vars).map(TypeHandle::Struct)?,
            TypeHandle::Generic(g) => type_vars
                .get(g)
                .ok_or(RuntimeError::FailedMakeGeneric.throw())?
                .clone(),
            Self::Unloaded(r) => return Err(RuntimeError::UnloadedType(r.clone()).throw().into()),
        };
        self.assembly().unwrap().add_type(this.clone());
        Ok(this)
    }
    pub fn type_vars(&self) -> Arc<IndexMap<StringName, TypeVar>> {
        match self {
            TypeHandle::Class(class) => class.type_vars().clone(),
            TypeHandle::Interface(interface) => interface.type_vars().clone(),
            TypeHandle::Struct(s) => s.type_vars().clone(),
            TypeHandle::Generic(_) => Default::default(),
            TypeHandle::Unloaded(_) => Default::default(),
        }
    }
}
