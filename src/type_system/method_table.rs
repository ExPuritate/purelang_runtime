use super::{
    AssemblyManager, Class, CommonMethod, Interface, Struct, TypeHandle,
    get_traits::{GetAssemblyMust, GetTypeName},
};
use crate::type_system::get_traits::{GetFieldCount, GetMethodTable, MTGetParent};
use export::AssemblyTrait;
use global::{
    Error, IndexMap, Result, StringMethodReference, StringName, ThreadSafe, errors::RuntimeError,
};
use std::{
    any::Any,
    sync::{Arc, Weak},
};

#[derive(ThreadSafe, derive_more::Debug)]
pub struct CommonMethodTable<T>
where
    T: Any + GetTypeName,
{
    pub(crate) map: IndexMap<StringName, CommonMethod<T>>,
    #[debug(skip)]
    pub(crate) t: Weak<T>,
    #[debug("{:#?}", parent.as_ref().map(|x| x.name().clone()))]
    pub(crate) parent: Option<TypeHandle>,
    pub(crate) field_count: u64,
}

impl<T: Any + GetTypeName> Clone for CommonMethodTable<T> {
    fn clone(&self) -> Self {
        Self {
            map: self.map.clone(),
            t: self.t.clone(),
            parent: self.parent.clone(),
            field_count: self.field_count,
        }
    }
}

impl<T: Any + GetTypeName> CommonMethodTable<T> {
    pub(crate) fn ty(&self) -> Arc<T> {
        self.t.upgrade().unwrap()
    }
}

#[allow(private_bounds)]
impl<T: Any + GetTypeName + GetFieldCount> CommonMethodTable<T> {
    pub fn new<F: Fn(*mut Self) -> IndexMap<StringName, CommonMethod<T>>>(
        map_generator: F,
        t: &Arc<T>,
        parent: Option<TypeHandle>,
    ) -> *mut Self {
        Self::try_new(|p| Ok(map_generator(p)), t, parent).unwrap()
    }
    pub fn try_new<F: Fn(*mut Self) -> global::Result<IndexMap<StringName, CommonMethod<T>>>>(
        map_generator: F,
        t: &Arc<T>,
        parent: Option<TypeHandle>,
    ) -> global::Result<*mut Self> {
        let this = Box::leak(Box::new(Self {
            map: IndexMap::new(),
            t: Arc::downgrade(t),
            parent,
            field_count: t.field_count() as _,
        }));
        let ptr = this as *mut _;
        let map = map_generator(ptr)?;
        this.map = map;
        Ok(ptr)
    }
}

impl<T: Any + GetTypeName + GetAssemblyMust + GetMethodTable> CommonMethodTable<T>
where
    Self: MTGetParent<T>,
{
    pub fn get_method(&self, method_ref: &StringMethodReference) -> Result<CommonMethod<T>> {
        match method_ref {
            StringMethodReference::Single(name) => self
                .map
                .get(name)
                .cloned()
                .or_else(|| {
                    unsafe { &*self._parent()?.mt_ptr() }
                        .get_method(method_ref)
                        .ok()
                })
                .ok_or(RuntimeError::FailedGetMethod(method_ref.clone()).into()),
            StringMethodReference::WithGeneric(name, type_vars) => {
                match self.map.get(name).map(|x| {
                    x.make_generic(Arc::new(
                        type_vars
                            .iter()
                            .map(|x| {
                                Ok::<_, Error>((
                                    x.0.clone(),
                                    AssemblyManager::from_dyn(self.ty().must_assembly().manager())
                                        .get_type_from_str(x.1)?,
                                ))
                            })
                            .try_collect::<IndexMap<_, _>>()?,
                    ))
                }) {
                    Some(m) => m,
                    None => Err(RuntimeError::FailedGetMethod(method_ref.clone()).into()),
                }
            }
        }
    }
}

impl<T: Any + GetTypeName> CommonMethodTable<T> {
    pub fn make_generic(&self, type_vars: Arc<IndexMap<StringName, TypeHandle>>) -> Result<Self> {
        Ok(Self {
            map: self
                .map
                .iter()
                .map(|x| Ok::<_, Error>((x.0.clone(), x.1.make_generic(type_vars.clone())?)))
                .try_collect::<IndexMap<_, _>>()?,

            t: self.t.clone(),
            parent: self.parent.clone(),
            field_count: self.field_count,
        })
    }
}

impl CommonMethodTable<Class> {
    pub fn class(&self) -> Arc<Class> {
        self.ty()
    }
}

impl CommonMethodTable<Interface> {
    pub fn interface(&self) -> Arc<Interface> {
        self.ty()
    }
}

impl CommonMethodTable<Struct> {
    pub fn struct_type(&self) -> Arc<Struct> {
        self.ty()
    }
}
