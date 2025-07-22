#![allow(nonstandard_style)]

use crate::pl_lib_impl::ClassLoadToCore;
use crate::type_system::{
    Assembly, AssemblyManager, Class, CommonMethod, CommonMethodTable, TypeHandle,
};
use crate::value::{ByRefValue, StringValue, Value};
use crate::vm::CPU;
use enumflags2::make_bitflags;
use gc::Gc;
use global::attrs::{
    ClassImplementationFlags, MethodAttr, MethodImplementationFlags, TypeAttr, TypeSpecificAttr,
    Visibility,
};
use global::errors::{DynamicCheckingItem, RuntimeError};
use global::{StringMethodReference, StringTypeReference, indexmap, string_name};
use std::sync::Arc;

pub struct System_Object;

impl System_Object {
    fn ToString(
        _: &CommonMethod<Class>,
        cpu: Arc<CPU>,
        this_val: &mut Value,
        args: &mut [Value],
        _register_start: u64,
    ) -> global::Result<Value> {
        if cpu.vm().is_dynamic_checking_enabled() && !args.is_empty() {
            return Err(
                RuntimeError::DynamicCheckingFailed(DynamicCheckingItem::ArgLen {
                    got: args.len(),
                    expected: 0,
                })
                .throw()
                .into(),
            );
        }
        let s = this_val.ty(cpu)?.string_reference().string_name_repr();
        Ok(Value::Reference(Gc::new(ByRefValue::String(
            StringValue::new(s.as_str().to_owned()),
        ))))
    }
}

impl ClassLoadToCore for System_Object {
    const STRING_TYPE_REFERENCE: StringTypeReference =
        StringTypeReference::core_static_single_type("System.Object");
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
                            string_name!(".ctor()") => CommonMethod::native(
                                string_name!(".ctor()"),
                                MethodAttr::new(Visibility::Public, make_bitflags!(MethodImplementationFlags::{}), 0),
                                mt_ptr,
                                TypeHandle::Unloaded(AssemblyManager::System_Void_STRUCT_REF),
                                vec![],
                                Default::default(),
                                |_, _, _, _, _| Ok(Value::Void),
                            ),
                            StringMethodReference::STATIC_CTOR_REF.unwrap_single() => CommonMethod::native(
                                StringMethodReference::STATIC_CTOR_REF.unwrap_single(),
                                MethodAttr::new(Visibility::Public, make_bitflags!(MethodImplementationFlags::{}), 0),
                                mt_ptr,
                                TypeHandle::Unloaded(AssemblyManager::System_Void_STRUCT_REF),
                                vec![],
                                Default::default(),
                                |_, _, _, _, _| Ok(Value::Void),
                            ),
                            string_name!("ToString()") => CommonMethod::native(
                                string_name!("ToString()"),
                                MethodAttr::new(Visibility::Public, make_bitflags!(MethodImplementationFlags::{}), 0),
                                mt_ptr,
                                TypeHandle::Unloaded(StringTypeReference::make_static_single("!", "System.String")),
                                vec![],
                                Default::default(),
                                Self::ToString,
                            ),
                        }
                    },
                    &class,
                    None,
                )
            },
            indexmap! {},
        );
        core_assembly.add_type(TypeHandle::Class(class));
    }
}
