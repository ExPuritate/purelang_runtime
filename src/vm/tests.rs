#![allow(unused_variables)]

use crate::pl_lib_impl::ClassLoadToCore;
use std::{
    fmt::{self, FormattingOptions},
    io::Cursor,
    sync::{Arc, LazyLock},
};

use enumflags2::make_bitflags;
use export::AssemblyManagerTrait;
use global::{
    Result,
    attrs::{
        ClassImplementationFlags, FieldAttr, FieldImplementationFlags, MethodAttr,
        MethodImplementationFlags, TypeAttr, TypeSpecificAttr, Visibility,
    },
    indexmap,
    instruction::StringInstruction,
    string_name,
};

use crate::pl_lib_impl::System_Object::System_Object;
use crate::{
    type_system::{Class, ClassField, CommonMethod, CommonMethodTable},
    value::{Array, Value},
};

static GENERAL_VM: LazyLock<Arc<VM>> = LazyLock::new(|| {
    let vm = VM::with_config(VMConfig::builder().build()).unwrap();
    vm.clone().load_statics().unwrap();
    vm
});

use super::*;

#[test]
fn test_vm_cpu() -> Result<()> {
    let vm = GENERAL_VM.clone();
    let (i, cpu) = vm.clone().new_cpu();
    let cpu = CPU::from_dyn(cpu);
    assert_eq!(i, cpu.id());
    let gotten_cpu = CPU::from_dyn(vm.get_cpu(i as usize).unwrap());
    assert_eq!(cpu.id(), gotten_cpu.id());
    assert_eq!(
        cpu.register_len() as u64,
        vm.config
            .read()
            .unwrap()
            .default_cpu_config()
            .default_register_num()
    );
    Ok(())
}

#[test]
fn test_vm_run() -> Result<()> {
    let vm = GENERAL_VM.clone();
    let assem_mgr = AssemblyManager::from_dyn(vm.assembly_manager());
    let assem_mgr = &assem_mgr;
    let assem = Arc::new(Assembly::new(string_name!("Test"), assem_mgr));
    assem.add_type(TypeHandle::Class(Class::new(
        &assem,
        TypeAttr::new(
            Visibility::Public,
            TypeSpecificAttr::Class(make_bitflags!(ClassImplementationFlags::{})),
        ),
        string_name!("Test.Test"),
        |class| {
            CommonMethodTable::new(
                |mt_ptr| indexmap! {
                    string_name!("Main([!]System.Array`1[[!]System.String])") => CommonMethod::new(
                        string_name!("Main([!]System.Array`1[[!]System.String])"),
                        MethodAttr::new(Visibility::Public, make_bitflags!(MethodImplementationFlags::{}), 10),
                        mt_ptr, vec![
                            StringInstruction::LoadArg { arg: 0, register_addr: 0 },
                            StringInstruction::Load_u64 { register_addr: 1, val: 1 },
                            StringInstruction::InstanceCall {
                                val: 0,
                                method: StringMethodReference::Single(string_name!("__op_Index([!]System.UInt64)")),
                                args: vec![1],
                                ret_at: 2,
                            },
                            StringInstruction::StaticCall {
                                ty: StringTypeReference::core_static_single_type("System.Console"),
                                method: StringMethodReference::Single(string_name!("WriteLine([!]System.String)")),
                                args: vec![2],
                                ret_at: 3,
                            },
                        ].into(),
                        assem_mgr.get_type_from_str(&AssemblyManager::System_Void_STRUCT_REF).unwrap(),
                        vec![
                            vm.get_core_generic_type(string_name!("System.Array`1"), Arc::new(indexmap! {
                                string_name!("@T") => StringTypeReference::core_static_single_type("System.String")
                            })).unwrap(),
                        ],
                        Default::default(),
                    )
                },
                &class,
                Some(
                    assem_mgr
                        .get_type_from_str(&AssemblyManager::System_Object_CLASS_REF)
                        .unwrap(),
                ),
            ).cast()
        },
        indexmap! {},
    )));
    assem_mgr.add_assembly(assem);
    let (i, cpu) = vm.clone().new_cpu();
    let cpu = CPU::from_dyn(cpu);
    cpu.run(
        string_name!("Test"),
        string_name!("Test.Test"),
        vec!["aaa".to_owned(), "bbb".to_owned()],
    )?;
    Ok(())
}

#[test]
fn test_to_string() -> Result<()> {
    let vm = GENERAL_VM.clone();
    let assem_mgr = AssemblyManager::from_dyn(vm.assembly_manager());
    let assem_mgr = &assem_mgr;
    let assem = Arc::new(Assembly::new(string_name!("Test"), assem_mgr));
    assem.add_type(TypeHandle::Class(Class::new(
        &assem,
        TypeAttr::new(
            Visibility::Public,
            TypeSpecificAttr::Class(make_bitflags!(ClassImplementationFlags::{})),
        ),
        string_name!("Test.Test"),
        |class| {
            CommonMethodTable::new(
                |mt_ptr| indexmap! {
                    string_name!("Main([!]System.Array`1[[!]System.String])") => CommonMethod::new(
                        string_name!("Main([!]System.Array`1[[!]System.String])"),
                        MethodAttr::new(Visibility::Public, make_bitflags!(MethodImplementationFlags::{}), 10),
                        mt_ptr, vec![
                            StringInstruction::LoadAllArgsAsArray { register_addr: 0 },
                            StringInstruction::InstanceCall {
                                val: 0,
                                method: StringMethodReference::Single(string_name!("ToString()")),

                                args: vec![],
                                ret_at: 1,
                            },
                            StringInstruction::StaticCall {
                                ty: StringTypeReference::core_static_single_type("System.Console"),
                                method: StringMethodReference::Single(string_name!("WriteLine([!]System.String)")),

                                args: vec![1],
                                ret_at: 3,
                            }
                        ].into(),
                        assem_mgr.get_type_from_str(&AssemblyManager::System_Void_STRUCT_REF).unwrap(),
                        vec![
                            vm.get_core_generic_type(string_name!("System.Array`1"), Arc::new(indexmap! {
                                string_name!("@T") => StringTypeReference::core_static_single_type("System.String")
                            })).unwrap(),
                        ],
                        Default::default(),
                    )
                },
                &class,
                Some(
                    assem_mgr
                        .get_type_from_str(&AssemblyManager::System_Object_CLASS_REF)
                        .unwrap(),
                ),
            ).cast()
        },
        indexmap! {},
    )));
    assem_mgr.add_assembly(assem);
    let (i, cpu) = vm.clone().new_cpu();
    let cpu = CPU::from_dyn(cpu);
    cpu.run(
        string_name!("Test"),
        string_name!("Test.Test"),
        vec!["aaa".to_owned(), "bbb".to_owned()],
    )?;
    Ok(())
}

#[test]
#[allow(non_snake_case)]
fn test_static() -> Result<()> {
    const TEST_CLASS_NAME: StringTypeReference =
        StringTypeReference::make_static_single("Test", "Test.StaticTest");
    let out = Arc::new(RwLock::new(Cursor::new(Vec::<u8>::new())));
    let vm = VM::with_config(VMConfig::builder().build())?;
    let assem_mgr = AssemblyManager::from_dyn(vm.assembly_manager());
    let assem_mgr = &assem_mgr;
    let assem = Arc::new(Assembly::new(string_name!("Test"), assem_mgr));
    fn Test_StaticTest_sctor(
        method: &CommonMethod<Class>,
        cpu: Arc<CPU>,
        this: &mut Value,
        args: &mut [Value],
        reg_start: u64,
    ) -> Result<Value> {
        let this_v = this.unwrap_reference_mut()?.0;
        let (this_val,) = this_v.unwrap_object_mut()?;
        let mut val = cpu.create_object::<Class>(
            None,
            &StringTypeReference::core_static_single_type("System.String"),
            string_name!(".ctor()"),
            &mut [],
        )?;
        val.unwrap_string_mut()?.0.set("aaa".to_owned());
        this_val
            .get_mut_field(StringName::from("__test"))?
            .set_val(&Value::Reference(val));
        Ok(Value::Void)
    }
    assem.add_type(TypeHandle::Class(Class::new(
        &assem,
        TypeAttr::new(
            Visibility::Public,
            TypeSpecificAttr::Class(make_bitflags!(ClassImplementationFlags::{Static})),
        ),
        string_name!("Test.StaticTest"),
        |class| {
            CommonMethodTable::new(
                |table| {
                    indexmap! {
                        string_name!(".sctor()") => CommonMethod::native(
                            string_name!(".sctor()"),
                            MethodAttr::new(
                                Visibility::Public,
                                make_bitflags!(MethodImplementationFlags::{Static}),
                                0,
                            ),
                            table,
                            assem_mgr.get_type_from_str(&AssemblyManager::System_Void_STRUCT_REF).unwrap(),
                            vec![],
                            Default::default(),
                            Test_StaticTest_sctor,
                        ),
                        string_name!("PrintStatics()") => CommonMethod::new(
                            string_name!("PrintStatics()"),
                            MethodAttr::new(
                                Visibility::Public,
                                make_bitflags!(MethodImplementationFlags::{Static}),
                                2
                            ),
                            table,
                            vec![
                                StringInstruction::LoadStatic {
                                    register_addr: 0,
                                    ty: TEST_CLASS_NAME.clone(),
                                    name: string_name!("__test"),
                                },
                                StringInstruction::StaticCall {
                                    ty: StringTypeReference::core_static_single_type("System.Console"),
                                    method: StringMethodReference::Single(string_name!("WriteLine([!]System.String)")),
                                    args: vec![0],
                                    ret_at: 1,
                                },
                            ].into(),
                            assem_mgr.get_type_from_str(&AssemblyManager::System_Void_STRUCT_REF).unwrap(),
                            vec![],
                            Default::default(),
                        ),
                    }
                },
                &class,
                Some(
                    assem_mgr
                        .get_type_from_str(&AssemblyManager::System_Object_CLASS_REF)
                        .unwrap(),
                ),
            )
            .cast()
        },
        indexmap! {
            string_name!("__test") => ClassField::new(
                string_name!("__test"),
                FieldAttr::new(
                    Visibility::Private,
                    make_bitflags!(FieldImplementationFlags::{Static}),
                ),
                assem_mgr.get_type_from_str(&StringTypeReference::core_static_single_type("System.String")).unwrap()
            ),
        },
    )));
    assem_mgr.add_assembly(assem.clone());
    let ty = assem
        .get_type(&StringTypeReference::make_static_single(
            "Test",
            "Test.StaticTest",
        ))?
        .unwrap_class();
    dbg!(ty.static_fields());
    vm.clone().load_statics()?;
    let val = vm.clone().get_static_from_str(
        &StringTypeReference::make_static_single("Test", "Test.StaticTest"),
        "__test",
    )?;
    let val = val.unwrap_reference_ref()?.0;
    let val = val.unwrap_string_ref()?.0;
    dbg!(val);
    let (cpu_id, cpu) = vm.clone().new_cpu();
    let cpu = CPU::from_dyn(cpu);
    cpu.call_static_str_method(
        &TEST_CLASS_NAME,
        &StringMethodReference::static_single("PrintStatics()"),
        &mut [],
    )?;
    Ok(())
}

#[test]
fn test_array() -> Result<()> {
    let vm = VM::new()?;
    let (i, cpu) = vm.clone().new_cpu();
    let cpu = CPU::from_dyn(cpu);
    let mut arr = Array::alloc(
        cpu.clone(),
        vm.get_type(&StringTypeReference::Single {
            assem: string_name!("!"),
            ty: string_name!("System.Object"),
        })?,
    );
    let (array,) = arr.unwrap_array_mut()?;
    dbg!(array.ty(vm.clone()).name());
    array.grow_to(10);
    for i in 0..10usize {
        array[i] = Value::UInt64(i as _);
    }
    let mut s = String::new();
    let mut formatter = fmt::Formatter::new(&mut s, *FormattingOptions::new().alternate(true));
    array.dbg_fmt(vm.clone(), &mut formatter)?;
    eprintln!("{s}");
    Ok(())
}
#[test]
fn test_from_ir() -> Result<()> {
    let vm = GENERAL_VM.clone();
    vm.assembly_manager()
        .clone()
        .load_from_binary_assemblies(&[binary::Assembly::from_file("./test.plb")?])?;

    dbg!(
        AssemblyManager::from_dyn(vm.assembly_manager())
            .get_type_from_str(&StringTypeReference::Single {
                assem: string_name!("Test"),
                ty: string_name!("Test.StaticTest"),
            })?
            .unwrap_class_ref()
            .mt()
            .parent
            .unwrap()
            .unwrap_class_ref()
            .mt()
    );
    vm.clone().load_statics()?;
    let res = CPU::from_dyn(vm.new_cpu().1).run(
        string_name!("Test"),
        string_name!("Test.StaticTest"),
        vec![],
    )?;
    Ok(())
}
