use super::{Assembly, CommonMethodTable, TypeHandle, TypeVar};
use export::AssemblyTrait;
use global::derive_ctor::ctor;
use global::getset::CopyGetters;
use global::{
    IndexMap, Result, StringName, StringTypeReference, ThreadSafe,
    attrs::{FieldAttr, TypeAttr},
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
pub struct Struct {
    #[debug(skip)]
    #[getset(skip)]
    assem: Weak<Assembly>,
    #[getset(skip)]
    #[get_copy = "pub"]
    attr: TypeAttr,
    name: StringName,
    general_name: StringName,
    #[getset(skip)]
    pub(crate) mt: Cell<*mut CommonMethodTable<Self>>,
    fields: IndexMap<StringName, Field>,
    #[debug("{:#?}", type_vars.iter().map(|x| (x.0, x.1.name())).collect::<IndexMap<_, _>>())]
    pub(crate) type_vars: Arc<IndexMap<StringName, TypeVar>>,
}

impl Struct {
    pub fn try_new<F: FnOnce(Arc<Struct>) -> global::Result<*mut CommonMethodTable<Self>>>(
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
        this.mt.set(mt.cast());
        Ok(this)
    }

    pub fn new<F: FnOnce(Arc<Struct>) -> *mut CommonMethodTable<Self>>(
        assem: &Arc<Assembly>,
        attr: TypeAttr,
        name: StringName,
        mt_generator: F,
        fields: IndexMap<StringName, Field>,
    ) -> Arc<Self> {
        let this = Arc::new(Self {
            assem: Arc::downgrade(assem),
            attr,
            name: name.clone(),
            general_name: name,
            mt: Cell::new(ptr::null_mut()),
            fields,
            type_vars: Arc::new(IndexMap::new()),
        });
        let mt = mt_generator(this.clone());
        assert!(!mt.is_null());
        this.mt.set(mt.cast());
        this
    }
}

impl Struct {
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
            return Ok(this.unwrap_struct());
        }
        let mt = Box::leak(Box::new(self.mt().make_generic(type_vars.clone())?));
        let type_vars = Arc::new(
            type_vars
                .iter()
                .map(|(k, v)| (k.clone(), TypeVar::Type(v.clone())))
                .collect::<IndexMap<_, _>>(),
        );
        let this = Arc::new(Self {
            assem: self.assem.clone(),
            attr: self.attr,
            name,
            general_name: self.general_name.clone(),
            mt: Cell::new(mt),
            fields: self.fields.clone(),
            type_vars,
        });
        Ok(this)
    }
}

impl Struct {
    pub fn mt(&self) -> CommonMethodTable<Self> {
        unsafe { (*self.mt.get()).clone() }
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
                        .filter_map(|(a, b)| {
                            Some((a.clone(), {
                                if let TypeVar::Type(x) = b {
                                    x.string_reference()
                                } else {
                                    return None;
                                }
                            }))
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
    pub fn static_fields(&self) -> IndexMap<StringName, Field> {
        self.fields()
            .into_iter()
            .filter(|x| x.1.attr.is_static())
            .map(|x| (x.0.clone(), x.1.clone()))
            .collect()
    }
}

#[derive(Clone, Getters, CopyGetters, derive_more::Debug, ctor, ThreadSafe)]
#[getset(get = "pub")]
pub struct Field {
    name: StringName,
    #[getset(skip)]
    #[get_copy = "pub"]
    attr: FieldAttr,
    #[debug("{:#?}", ty.name())]
    ty: TypeHandle,
}
