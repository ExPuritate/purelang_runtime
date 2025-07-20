mod assembly;
mod class;
pub mod get_traits;
mod interface;
mod manager;
mod method;
mod method_table;
mod r#struct;
#[cfg(test)]
mod tests;
mod type_handle;

pub use assembly::Assembly;
pub use class::{Class, Field as ClassField};
use global::StringName;
pub use interface::Interface;
pub use manager::AssemblyManager;
pub use method::CommonMethod;
use std::fmt::{Debug, Formatter};
pub use r#struct::{Field as StructField, Struct};
pub use type_handle::TypeHandle;

pub(crate) use method_table::CommonMethodTable;

#[derive(Clone)]
pub struct GenericBinding {
    pub(crate) implemented_interfaces: Vec<TypeHandle>,
    pub(crate) parent: Option<TypeHandle>,
}

#[derive(Clone)]
pub enum TypeVar {
    Type(TypeHandle),
    Canon(GenericBinding),
}

impl Debug for TypeVar {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            TypeVar::Type(t) => write!(f, "TypeVar::Type({:?})", t.string_reference()),
            TypeVar::Canon(_) => write!(f, "TypeVar::Canon(System.__Canon)"),
        }
    }
}

impl TypeVar {
    pub fn name(&self) -> StringName {
        match self {
            Self::Type(t) => t.name().clone(),
            Self::Canon(_) => StringName::from_static_str("System.__Canon"),
        }
    }
}
