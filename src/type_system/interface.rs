use export::AssemblyTrait;
use global::{IndexMap, StringName, StringTypeReference, getset::Getters};
use std::cell::Cell;
use std::sync::{Arc, Weak};

use super::{Assembly, CommonMethodTable, TypeVar};

#[derive(Getters)]
#[getset(get = "pub")]
pub struct Interface {
    #[getset(skip)]
    assem: Weak<Assembly>,
    name: StringName,
    general_name: StringName,
    #[getset(skip)]
    pub(crate) mt: Cell<*mut CommonMethodTable<Self>>,
    pub(crate) type_vars: Arc<IndexMap<StringName, TypeVar>>,
}

impl Interface {
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
}
