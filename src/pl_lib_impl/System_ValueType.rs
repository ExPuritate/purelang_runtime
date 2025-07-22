use crate::pl_lib_impl::StructLoadToCore;
use crate::type_system::{Assembly, AssemblyManager, CommonMethodTable, Struct, TypeHandle};
use crate::type_system::CommonMethod;
use crate::value::{ByRefValue, StringValue, Value};
use crate::vm::CPU;
use enumflags2::make_bitflags;
use gc::Gc;
use global::StringMethodReference;
use global::attrs::{MethodAttr, MethodImplementationFlags};
use global::attrs::{StructImplementationFlags, TypeAttr, TypeSpecificAttr, Visibility};
use global::errors::{DynamicCheckingItem, RuntimeError};
use global::string_name;
use global::{StringTypeReference, indexmap};
use std::sync::Arc;

pub struct System_ValueType;

impl System_ValueType {
    fn ToString(
        _: &CommonMethod<Struct>,
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

impl StructLoadToCore for System_ValueType {
    const STRING_TYPE_REFERENCE: StringTypeReference =
        StringTypeReference::core_static_single_type("System.ValueType");

    fn load_struct(core_assembly: &Arc<Assembly>, _: &AssemblyManager) {
        let r#struct = Struct::new(
            core_assembly,
            TypeAttr::new(
                Visibility::Public,
                TypeSpecificAttr::Struct(make_bitflags!(StructImplementationFlags::{})),
            ),
            Self::unwrap_single_name_of_str_type_ref(),
            |s| {
                CommonMethodTable::new(
                    |mt_ptr| {
                        indexmap! {
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
                    &s,
                    None,
                )
            },
            indexmap! {},
        );
        core_assembly.add_type(TypeHandle::Struct(r#struct));
    }
}
