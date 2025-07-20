use crate::type_system::{Assembly, AssemblyManager};
use std::sync::Arc;

macro make_integers(
    $($i:ident),+ $(,)?
    #core_assembly: $core_assembly:expr;
    #assembly_manager: $assembly_manager:expr;
) {$(
    ::paste::paste! {
        struct [<System_ $i>];

        impl [<System_ $i>] {}

        impl super::StructLoadToCore for [<System_ $i>] {
            const STRING_TYPE_REFERENCE: ::global::StringTypeReference =
                ::global::StringTypeReference::core_single_type(
                    Self::TYPE_NAME
                );

            fn load_struct(
                core_assembly: &::std::sync::Arc<$crate::type_system::Assembly>,
                _assembly_manager: &$crate::type_system::AssemblyManager
            ) {
                use global::attrs::{
                    StructImplementationFlags, TypeAttr, Visibility, TypeSpecificAttr,
                };
                let r#struct = $crate::type_system::Struct::new(
                    &core_assembly,
                    TypeAttr::new(
                        Visibility::Public,
                        TypeSpecificAttr::Struct(::enumflags2::make_bitflags!(StructImplementationFlags::{})),
                    ),
                    Self::TYPE_NAME,
                    |s| {
                        $crate::type_system::CommonMethodTable::new(
                            |_mt_ptr| ::global::IndexMap::new(),
                            &s,
                            Some(core_assembly
                                .get_type(&super::System_ValueType::System_ValueType::STRING_TYPE_REFERENCE)
                                .unwrap(),
                            ),
                        )
                    },
                    ::global::indexmap! {},
                );
                core_assembly.add_type($crate::type_system::TypeHandle::Struct(r#struct));
            }
        }

        impl [<System_ $i>] {
            pub const TYPE_NAME: ::global::StringName = ::global::StringName::from_static_str(::const_format::formatcp!("System.{}", stringify!($i)));
        }

        [<System_ $i>]::load_struct($core_assembly, $assembly_manager);
    }
)+}

pub fn load_integers(core_assembly: &Arc<Assembly>, assembly_manager: &AssemblyManager) {
    #[allow(unused)]
    use super::StructLoadToCore;
    make_integers! {
        UInt8,
        UInt16,
        UInt32,
        UInt64,
        UInt128,
        Int8,
        Int16,
        Int32,
        Int64,
        Int128,
        #core_assembly: core_assembly;
        #assembly_manager: assembly_manager;
    }
}
