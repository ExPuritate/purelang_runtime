#![allow(nonstandard_style)]

use crate::pl_lib_impl::ClassLoadToCore;
use crate::pl_lib_impl::System_Object::System_Object;
use crate::type_system::{
    Assembly, AssemblyManager, Class, CommonMethod, CommonMethodTable, TypeHandle,
};
use crate::value::Value;
use crate::vm::CPU;
use enumflags2::make_bitflags;
use global::attrs::{
    ClassImplementationFlags, MethodAttr, MethodImplementationFlags, TypeAttr, TypeSpecificAttr,
    Visibility,
};
use global::errors::RuntimeError;
use global::{IndexMap, StringTypeReference, indexmap, string_name};
use std::sync::Arc;

pub struct System_Array;

impl System_Array {
    /// Sign: `__op_Index([!]System.UInt64)`
    fn __op_Index(
        _method: &CommonMethod<Class>,
        cpu: Arc<CPU>,
        this_val: &mut Value,
        args: &mut [Value],
        _register_start: u64,
    ) -> global::Result<Value> {
        if cpu.vm().is_dynamic_checking_enabled() {
            #[allow(clippy::unused_unit)]
            ()
        }
        let Value::UInt64(arg0) = &args[0] else {
            return Err(RuntimeError::WrongType.into());
        };
        let Value::Reference(this) = this_val else {
            return Err(RuntimeError::WrongType.into());
        };
        let (this_arr,) = this.unwrap_array_ref()?;
        Ok(this_arr
            .get(*arg0 as usize)
            .ok_or(RuntimeError::ArrayIndexOutOfRange)?
            .clone())
    }
}

impl ClassLoadToCore for System_Array {
    const STRING_TYPE_REFERENCE: StringTypeReference =
        StringTypeReference::core_single_type(string_name!("System.Array`1"));
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
                        indexmap! {
                        string_name!("__op_Index([!]System.UInt64)") => CommonMethod::native(
                            string_name!("__op_Index([!]System.UInt64)"),
                            MethodAttr::new(Visibility::Public, make_bitflags!(MethodImplementationFlags::{}), 1),
                            mt_ptr,
                            TypeHandle::Generic(string_name!("@T")),
                            vec! [
                                TypeHandle::Unloaded(
                                    StringTypeReference::core_static_single_type("System.UInt64"),
                                ),
                            ],
                            Default::default(),
                            Self::__op_Index,
                        )
                    }
                    },
                    &class,
                    Some(core_assembly
                             .get_type(&System_Object::STRING_TYPE_REFERENCE)
                             .unwrap(),
                    ),
                ).cast()
            },
            IndexMap::new(),
        );
        core_assembly.add_type(TypeHandle::Class(class));
    }
}
