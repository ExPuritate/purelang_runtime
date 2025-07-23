mod cpu;
#[cfg(test)]
mod tests;

use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

use crate::type_system::{Class, Struct};
use crate::value::StructObject;
use crate::{
    type_system::{Assembly, AssemblyManager, TypeHandle},
    value::{ByRefValue, Object, Value},
};
pub use cpu::CPU;
use export::{
    AssemblyManagerTrait, AssemblyTrait, CPUTrait, VMTrait, VMTrait_Assembly, VMTrait_CPU,
    VMTrait_Statics,
};
use gc::Gc;
use global::{
    Error, IndexMap, Result, StringMethodReference, StringName, StringTypeReference, ThreadSafe,
    configs::runtime::VMConfig, errors::RuntimeError, inline_all,
};

#[derive(ThreadSafe, Debug)]
pub struct VM {
    config: Arc<RwLock<VMConfig>>,
    cpus: Arc<RwLock<Vec<Arc<CPU>>>>,
    assembly_manager: Arc<AssemblyManager>,
    per_vm_statics_map: Arc<RwLock<HashMap<StringTypeReference, Value>>>,
}

impl VMTrait for VM {}

/// ctor methods
#[inline_all]
#[allow(clippy::arc_with_non_send_sync)]
impl VM {
    pub fn new() -> Result<Arc<Self>> {
        Self::with_config(Default::default())
    }
    pub fn with_config(config: VMConfig) -> Result<Arc<Self>> {
        Self::with_config_assembly_manager(config, AssemblyManager::new()?)
    }
    pub fn with_config_assembly_manager(
        config: VMConfig,
        assembly_manager: Arc<AssemblyManager>,
    ) -> Result<Arc<Self>> {
        Ok(Arc::new(Self {
            config: Arc::new(RwLock::new(config)),
            cpus: Arc::new(RwLock::new(Vec::with_capacity(1))),
            assembly_manager,
            per_vm_statics_map: Arc::new(RwLock::new(HashMap::new())),
        }))
    }
}

/// cpu
impl VMTrait_CPU for VM {
    fn new_cpu(self: Arc<Self>) -> (u64, Arc<dyn CPUTrait>) {
        let mut cpus = self.cpus.write().unwrap();
        let index = cpus.len() as u64;
        let cpu = CPU::new(self.clone(), index);
        cpus.push(cpu.clone());
        (index, cpu)
    }
    fn get_cpu(&self, index: usize) -> Option<Arc<dyn CPUTrait>> {
        self.cpus.read().unwrap().get(index).map(|x| {
            let x: Arc<dyn CPUTrait> = x.clone();
            x
        })
    }
}

impl VMTrait_Assembly for VM {
    fn get_core_assem(&self) -> Arc<dyn AssemblyTrait> {
        self.assembly_manager
            .get_assembly(StringTypeReference::CORE_ASSEMBLY_NAME)
            .unwrap()
    }
    fn get_assembly(&self, name: StringName) -> Result<Arc<dyn AssemblyTrait>> {
        self.assembly_manager.get_assembly(name)
    }
    fn assembly_manager(&self) -> Arc<dyn AssemblyManagerTrait> {
        self.assembly_manager.clone()
    }
    #[allow(clippy::arc_with_non_send_sync)]
    fn add_assembly_lookuper(&self, f: Arc<dyn Fn(&str) -> Option<String>>) {
        let mut config = self.config.write().unwrap();
        let origin = config.assembly_lookuper_mut();
        match origin {
            Some(f_origin) => {
                let _f = f_origin.clone();
                *f_origin = Arc::new(move |s| match _f(s) {
                    Some(out) => Some(out),
                    None => f(s),
                });
            }
            None => {
                *origin = Some(f);
            }
        }
    }
    fn add_assembly_dir(&self, p: &str) {
        let p = p.to_owned();
        self.add_assembly_lookuper(Arc::new(move |s| {
            for entry in std::fs::read_dir(&p).ok()? {
                let entry = entry.ok()?;
                if entry.file_type().unwrap().is_file() && *entry.path().as_os_str() == *s {
                    return Some(entry.path().to_str()?.to_owned());
                }
            }
            None
        }));
    }
}

impl VM {
    pub fn get_type(&self, type_ref: &StringTypeReference) -> Result<TypeHandle> {
        self.assembly_manager.get_type_from_str(type_ref)
    }
    pub fn get_core_single_type(&self, type_name: StringName) -> Result<TypeHandle> {
        Assembly::from_dyn(self.get_core_assem())
            .get_type(&StringTypeReference::core_single_type(type_name))
    }
    pub fn get_core_generic_type(
        &self,
        ty: StringName,
        type_vars: Arc<IndexMap<StringName, StringTypeReference>>,
    ) -> Result<TypeHandle> {
        Assembly::from_dyn(self.get_core_assem())
            .get_type(&StringTypeReference::core_single_type(ty))?
            .make_generic(Arc::new(
                type_vars
                    .iter()
                    .map(|x| Ok::<_, Error>((x.0.clone(), self.get_type(x.1)?)))
                    .try_collect::<IndexMap<_, _>>()?,
            ))
    }
    pub fn is_dynamic_checking_enabled(&self) -> bool {
        self.config.read().unwrap().is_dynamic_checking_enabled()
    }
    pub fn get_static_from_str(
        self: Arc<Self>,
        t: &StringTypeReference,
        name: &str,
    ) -> Result<Value> {
        let static_map = self.per_vm_statics_map.read().unwrap();
        let val = static_map
            .get(t)
            .ok_or(RuntimeError::FailedGetType(t.clone()))?;
        match val {
            Value::Void
            | Value::True
            | Value::False
            | Value::UInt8(_)
            | Value::UInt16(_)
            | Value::UInt32(_)
            | Value::UInt64(_)
            | Value::UInt128(_)
            | Value::Int8(_)
            | Value::Int16(_)
            | Value::Int32(_)
            | Value::Int64(_)
            | Value::Int128(_) => Err(RuntimeError::FailedGetField(name.into()).into()),
            Value::Struct(struct_object) => {
                Ok(struct_object.get_field(name).map(|x| x.val().clone())?)
            }
            Value::Reference(p) => match &**p {
                ByRefValue::Object(object) => Ok(object
                    .get_field(StringName::from(name))
                    .map(|x| x.val().clone())?),
                ByRefValue::Array(_) => Err(RuntimeError::UnsupportedGettingField.into()),
                ByRefValue::String(_) => Err(RuntimeError::UnsupportedGettingField.into()),
                ByRefValue::Null => todo!(),
            },
            Value::RegisterReference(_) => unimplemented!(),
        }
    }
}

impl VMTrait_Statics for VM {
    fn load_statics(self: Arc<Self>) -> global::Result<()> {
        let (_, cpu_static) = self.clone().new_cpu();
        let cpu_static = CPU::from_dyn(cpu_static);
        let assemblies = self.assembly_manager.all_types();
        let all_types = assemblies.values();
        let all_types = all_types.map(|x| x.values());
        for types in all_types {
            for t in types {
                match t {
                    TypeHandle::Class(class) => {
                        let fields = crate::value::object_get_static_fields(class.clone());
                        let mt = class.mt.get();
                        let obj = Object::internal_new(mt, fields);
                        let p = Gc::new(ByRefValue::Object(obj));
                        let mut reference = Value::Reference(p);
                        cpu_static.clone().call_instance_method::<Class>(
                            None,
                            &StringMethodReference::STATIC_CTOR_REF,
                            &mut reference,
                            &mut [],
                        )?;
                        self.per_vm_statics_map
                            .write()
                            .unwrap()
                            .insert(class.string_reference(), reference);
                    }
                    TypeHandle::Struct(s) => {
                        let fields = crate::value::struct_get_static_fields(s.clone());
                        let mt = s.mt.get();
                        let obj = StructObject::internal_new(mt, fields);
                        let mut reference = Value::Struct(obj);
                        cpu_static.clone().call_instance_method::<Struct>(
                            None,
                            &StringMethodReference::STATIC_CTOR_REF,
                            &mut reference,
                            &mut [],
                        )?;
                        self.per_vm_statics_map
                            .write()
                            .unwrap()
                            .insert(s.string_reference(), reference);
                    }
                    _ => continue,
                }
            }
        }
        Ok(())
    }
}

#[unsafe(no_mangle)]
#[allow(nonstandard_style)]
pub extern "Rust" fn NewVM() -> global::Result<Arc<dyn VMTrait>> {
    let vm: Arc<dyn VMTrait> = VM::new()?;
    Ok(vm)
}

#[unsafe(no_mangle)]
#[allow(nonstandard_style)]
pub extern "Rust" fn NewVMWithConfig(config: VMConfig) -> global::Result<Arc<dyn VMTrait>> {
    let vm: Arc<dyn VMTrait> = VM::with_config(config)?;
    Ok(vm)
}

#[unsafe(no_mangle)]
#[allow(nonstandard_style)]
pub extern "Rust" fn NewVMWithConfigAssemblyManager(
    config: VMConfig,
    assembly_manager: Arc<dyn AssemblyManagerTrait>,
) -> global::Result<Arc<dyn VMTrait>> {
    let vm: Arc<dyn VMTrait> =
        VM::with_config_assembly_manager(config, AssemblyManager::from_dyn(assembly_manager))?;
    Ok(vm)
}
