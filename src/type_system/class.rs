use super::{Assembly, CommonMethodTable, TypeHandle, TypeVar};
use export::AssemblyTrait;
use global::getset::CopyGetters;
use global::{
    IndexMap, Result, StringName, StringTypeReference, ThreadSafe,
    attrs::{FieldAttr, TypeAttr},
    derive_ctor::ctor,
    errors::RuntimeError,
    getset::Getters,
};
use std::{
    cell::Cell,
    ptr,
    sync::{Arc, Weak},
};

#[derive(Getters, ThreadSafe, derive_more::Debug, CopyGetters)]
#[getset(get = "pub")]
pub struct Class {
    #[debug(skip)]
    #[getset(skip)]
    assem: Weak<Assembly>,
    #[getset(skip)]
    #[get_copy = "pub"]
    attr: TypeAttr,
    name: StringName,
    general_name: StringName,
    #[getset(skip)]
    #[debug("{:#?}", self.mt())]
    pub(crate) mt: Cell<*mut CommonMethodTable<Self>>,
    fields: IndexMap<StringName, Field>,
    #[debug("{:#?}", type_vars.iter().map(|x| (x.0, x.1.name())).collect::<IndexMap<_, _>>())]
    pub(crate) type_vars: Arc<IndexMap<StringName, TypeVar>>,
}

impl Class {
    pub fn new<F: FnOnce(Arc<Class>) -> *mut CommonMethodTable<Self>>(
        assem: &Arc<Assembly>,
        attr: TypeAttr,
        name: StringName,
        mt_generator: F,
        fields: IndexMap<StringName, Field>,
    ) -> Arc<Self> {
        Self::try_new(assem, attr, name, |class| Ok(mt_generator(class)), fields).unwrap()
    }
    pub fn try_new<F: FnOnce(Arc<Class>) -> global::Result<*mut CommonMethodTable<Self>>>(
        assem: &Arc<Assembly>,
        attr: TypeAttr,
        name: StringName,
        mt_generator: F,
        fields: IndexMap<StringName, Field>,
    ) -> global::Result<Arc<Self>> {
        let this = Arc::new(Self {
            assem: Arc::downgrade(assem),
            attr,
            name: name.clone(),
            general_name: name,
            mt: Cell::new(ptr::null_mut()),
            fields,
            type_vars: Arc::new(IndexMap::new()),
        });
        let mt = mt_generator(this.clone())?;
        assert!(!mt.is_null());
        this.mt.set(mt);
        Ok(this)
    }
    pub(crate) fn new_inner(
        assem: Weak<Assembly>,
        attr: TypeAttr,
        name: StringName,
        general_name: StringName,
        mt: *mut CommonMethodTable<Self>,
        fields: IndexMap<StringName, Field>,
        type_vars: Arc<IndexMap<StringName, TypeVar>>,
    ) -> Self {
        Self {
            assem,
            attr,
            name,
            general_name,
            mt: Cell::new(mt),
            fields,
            type_vars,
        }
    }
}

impl Class {
    pub fn static_fields(&self) -> IndexMap<StringName, Field> {
        self.fields()
            .into_iter()
            .filter(|x| x.1.attr.is_static())
            .map(|x| (x.0.clone(), x.1.clone()))
            .collect()
    }
    pub fn assem(&self) -> Arc<Assembly> {
        self.assem.upgrade().unwrap()
    }
    pub fn string_reference(&self) -> StringTypeReference {
        if self.name.ne(&self.general_name) {
            StringTypeReference::WithGeneric {
                assem: self.assem().name(),
                ty: self.general_name.clone(),
                type_vars: Arc::new(
                    self.type_vars
                        .iter()
                        // TODO: Error when `b` is not `TypeVar::Type`
                        .filter_map(|(a, b)| {
                            Some((
                                a.clone(),
                                match b {
                                    TypeVar::Type(t) => t.string_reference(),
                                    TypeVar::Canon(_) => return None,
                                },
                            ))
                        })
                        .collect::<IndexMap<_, _>>(),
                ),
            }
        } else {
            StringTypeReference::Single {
                assem: self.assem().name(),
                ty: self.name.clone(),
            }
        }
    }
    pub fn general_string_reference(&self) -> StringTypeReference {
        StringTypeReference::Single {
            assem: self.assem().name(),
            ty: self.general_name.clone(),
        }
    }
}

impl Class {
    pub fn make_generic(
        self: Arc<Self>,
        type_vars: Arc<IndexMap<StringName, TypeHandle>>,
    ) -> Result<Arc<Self>> {
        if !self.name.contains('`') {
            return Err(RuntimeError::NonGenericType(self.name.clone()).into());
        }
        let name = StringTypeReference::WithGeneric {
            assem: self.assem().name(),
            ty: self.general_name.clone(),
            type_vars: Arc::new(
                type_vars
                    .iter()
                    .map(|(k, v)| (k.clone(), v.string_reference()))
                    .collect::<IndexMap<_, _>>(),
            ),
        }
        .string_name_repr();
        if let Ok(this) = self.assem().get_single_type(name.clone()) {
            return Ok(this.unwrap_class());
        }
        let mt = Box::leak(Box::new(self.mt().make_generic(type_vars.clone())?));
        let this = Arc::new(Self::new_inner(
            self.assem.clone(),
            self.attr,
            name,
            self.general_name.clone(),
            mt,
            self.fields.clone(),
            Arc::new(
                type_vars
                    .iter()
                    .map(|(k, v)| (k.clone(), TypeVar::Type(v.clone())))
                    .collect::<IndexMap<_, _>>(),
            ),
        ));
        Ok(this)
    }
}

impl Class {
    pub fn mt(&self) -> CommonMethodTable<Self> {
        unsafe { (*self.mt.get()).clone() }
    }
}

impl Drop for Class {
    fn drop(&mut self) {
        unsafe {
            if !self.mt.get().is_null() {
                self.mt.get().drop_in_place();
            }
        }
    }
}

#[derive(Clone, Getters, CopyGetters, derive_more::Debug, ctor)]
#[ctor(pub new)]
#[getset(get = "pub")]
pub struct Field {
    name: StringName,
    #[getset(skip)]
    #[get_copy = "pub"]
    attr: FieldAttr,
    #[debug("{:#?}", ty.name())]
    ty: TypeHandle,
}
