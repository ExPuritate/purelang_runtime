use super::{AssemblyManager, TypeHandle};
use export::{AssemblyManagerTrait, AssemblyTrait};
use global::{
    Error, IndexMap, Result, StringName, StringTypeReference, ThreadSafe, errors::RuntimeError,
};
use std::any::Any;
use std::{
    collections::HashMap,
    sync::{Arc, RwLock, Weak},
};

#[derive(Clone, ThreadSafe)]
pub struct Assembly {
    name: StringName,
    manager: Weak<AssemblyManager>,
    types: Arc<RwLock<HashMap<StringName, TypeHandle>>>,
}

impl Assembly {
    pub fn new(name: StringName, manager: &Arc<AssemblyManager>) -> Self {
        Self {
            name,
            manager: Arc::downgrade(manager),
            types: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    pub fn from_dyn(d: Arc<dyn AssemblyTrait>) -> Arc<Self> {
        unsafe { d.arc_any().downcast_unchecked() }
    }
}

impl Assembly {
    pub fn get_single_type(&self, name: StringName) -> Result<TypeHandle> {
        self.types.read().unwrap().get(&name).cloned().ok_or(
            RuntimeError::FailedGetType(StringTypeReference::Single {
                assem: self.name.clone(),
                ty: name,
            })
            .into(),
        )
    }
    pub fn get_type(&self, type_ref: &StringTypeReference) -> Result<TypeHandle> {
        self.get_type_from_str_complex(&|_| None, type_ref)
    }
    pub fn get_type_from_str_complex(
        &self,
        type_vars_lookup: &dyn Fn(&StringName) -> Option<TypeHandle>,
        type_ref: &StringTypeReference,
    ) -> Result<TypeHandle> {
        match type_ref {
            StringTypeReference::Single { assem, ty } => {
                if assem.eq(&self.name) {
                    self.types
                        .read()
                        .unwrap()
                        .get(ty)
                        .cloned()
                        .ok_or(RuntimeError::FailedGetType(type_ref.clone()).into())
                } else {
                    AssemblyManager::from_dyn(self.manager()).get_type_from_str(type_ref)
                }
            }
            StringTypeReference::Generic(generic) => {
                if let Some(ty) = type_vars_lookup(generic) {
                    Ok(ty)
                } else {
                    Err(RuntimeError::FailedGetType(type_ref.clone()).into())
                }
            }
            StringTypeReference::WithGeneric {
                assem,
                ty,
                type_vars,
            } => {
                if assem.eq(&self.name) {
                    let type_vars = Arc::new(
                        type_vars
                            .iter()
                            .map(|(n, t)| Ok::<_, Error>((n.clone(), self.get_type(t)?)))
                            .try_collect::<IndexMap<_, _>>()?,
                    );
                    match self
                        .types
                        .read()
                        .unwrap()
                        .get(ty)
                        .map(|x| x.make_generic(type_vars))
                    {
                        Some(e) => e,
                        None => Err(RuntimeError::FailedGetType(type_ref.clone()).into()),
                    }
                } else {
                    AssemblyManager::from_dyn(self.manager()).get_type_from_str(type_ref)
                }
            }
        }
    }
    pub fn types(&self) -> &Arc<RwLock<HashMap<StringName, TypeHandle>>> {
        &self.types
    }
    pub fn add_type(&self, ty: TypeHandle) {
        self.types.write().unwrap().insert(ty.name().clone(), ty);
    }
}

impl AssemblyTrait for Assembly {
    fn arc_any(self: Arc<Self>) -> Arc<dyn Any + Send + Sync> {
        self
    }
    fn name(&self) -> StringName {
        self.name.clone()
    }
    fn manager(&self) -> Arc<dyn AssemblyManagerTrait> {
        self.manager.upgrade().unwrap()
    }
}
