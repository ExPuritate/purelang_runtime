#![allow(nonstandard_style)]

use crate::pl_lib_impl::ClassLoadToCore;
use crate::pl_lib_impl::System_Object::System_Object;
use crate::type_system::{Assembly, AssemblyManager, Class, CommonMethodTable, TypeHandle};
use enumflags2::make_bitflags;
use global::attrs::{ClassImplementationFlags, TypeAttr, TypeSpecificAttr, Visibility};
use global::{IndexMap, StringName, StringTypeReference, string_name};
use std::sync::Arc;

pub struct System_Null;

impl System_Null {}

impl ClassLoadToCore for System_Null {
    const STRING_TYPE_REFERENCE: StringTypeReference =
        StringTypeReference::core_single_type(Self::type_name());
    fn load_class(core_assembly: &Arc<Assembly>, _: &AssemblyManager) {
        let class = Class::new(
            core_assembly,
            TypeAttr::new(
                Visibility::Public,
                TypeSpecificAttr::Class(make_bitflags!(ClassImplementationFlags::{})),
            ),
            Self::STRING_TYPE_REFERENCE.unwrap_single_name_ref().clone(),
            |class| {
                CommonMethodTable::new(
                    |_mt_ptr| IndexMap::new(),
                    &class,
                    Some(
                        core_assembly
                            .get_type(&System_Object::STRING_TYPE_REFERENCE)
                            .unwrap(),
                    ),
                )
            },
            IndexMap::new(),
        );
        core_assembly.add_type(TypeHandle::Class(class));
    }
}

impl System_Null {
    pub const fn type_name() -> StringName {
        string_name!("System.Null")
    }
}
