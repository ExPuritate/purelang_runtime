use crate::pl_lib_impl::StructLoadToCore;
use crate::pl_lib_impl::System_ValueType::System_ValueType;
use crate::type_system::{Assembly, AssemblyManager, CommonMethodTable, Struct, TypeHandle};
use enumflags2::make_bitflags;
use global::attrs::{StructImplementationFlags, TypeAttr, TypeSpecificAttr, Visibility};
use global::{IndexMap, StringName, StringTypeReference, indexmap, string_name};
use std::sync::Arc;

pub struct System_Boolean;

impl System_Boolean {}

impl StructLoadToCore for System_Boolean {
    const STRING_TYPE_REFERENCE: StringTypeReference =
        StringTypeReference::core_single_type(Self::type_name());
    fn load_struct(core_assembly: &Arc<Assembly>, _: &AssemblyManager) {
        let r#struct = Struct::new(
            core_assembly,
            TypeAttr::new(
                Visibility::Public,
                TypeSpecificAttr::Struct(make_bitflags!(StructImplementationFlags::{})),
            ),
            Self::type_name(),
            |s| {
                CommonMethodTable::new(
                    |_mt_ptr| IndexMap::new(),
                    &s,
                    Some(
                        core_assembly
                            .get_type(&System_ValueType::STRING_TYPE_REFERENCE)
                            .unwrap(),
                    ),
                )
            },
            indexmap! {},
        );
        core_assembly.add_type(TypeHandle::Struct(r#struct));
    }
}

impl System_Boolean {
    pub const fn type_name() -> StringName {
        string_name!("System.Boolean")
    }
}
