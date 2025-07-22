#![allow(nonstandard_style)]

use crate::pl_lib_impl::ClassLoadToCore;
use crate::type_system::{
    Assembly, AssemblyManager, Class, CommonMethod, CommonMethodTable, TypeHandle,
};
use crate::value::{ByRefValue, StringValue, Value};
use crate::vm::CPU;
use enumflags2::make_bitflags;
use global::attrs::{
    ClassImplementationFlags, MethodAttr, MethodImplementationFlags, TypeAttr, TypeSpecificAttr,
    Visibility,
};
use global::{IndexMap, StringTypeReference, lit_string_index_map, string_name};

use crate::pl_lib_impl::System_Object::System_Object;
use std::sync::Arc;

pub struct System_String;

impl System_String {
    fn ctor(
        _method: &CommonMethod<Class>,
        _cpu: Arc<CPU>,
        this_val: &mut Value,
        _args: &mut [Value],
        _register_start: u64,
    ) -> global::Result<Value> {
        let (this,) = this_val.unwrap_reference_mut()?;
        **this = ByRefValue::String(StringValue::new("".to_owned()));
        Ok(Value::Void)
    }
}

impl ClassLoadToCore for System_String {
    const STRING_TYPE_REFERENCE: StringTypeReference =
        StringTypeReference::core_single_type(string_name!("System.String"));
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
                    |mt_ptr| {
                        lit_string_index_map! {
                            // methods
                            ".ctor()" => CommonMethod::native(
                                string_name!(".ctor()"),
                                MethodAttr::new(Visibility::Public, make_bitflags!(MethodImplementationFlags::{}), 1),
                                mt_ptr,
                                core_assembly.get_single_type(string_name!("System.Void")).unwrap(),
                                vec![],
                                Default::default(),
                                Self::ctor,
                            )
                        }
                    },
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
