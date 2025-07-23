mod register;

use super::VM;
use crate::type_system::CommonMethod;
use crate::type_system::get_traits::GetAssemblyMust;
use crate::type_system::get_traits::GetTypeName;
use crate::type_system::get_traits::GetTypeVars;
use crate::{
    type_system::TypeHandle,
    value::{Array, ByRefValue, Object, StringValue, Value},
};
use export::CPUTrait;
use gc::{Gc, Trace};
use global::{
    Result, StringMethodReference, StringName, StringTypeReference, ThreadSafe,
    configs::runtime::CPUConfig, errors::RuntimeError, inline_all, string_name,
};
use register::RegisterGroup;
use std::any::Any;
use std::{cell::Cell, ptr, sync::Arc};

#[derive(ThreadSafe, derive_more::Debug)]
pub struct CPU {
    config: CPUConfig,
    id: u64,
    #[debug(skip)]
    vm: Arc<VM>,
    #[debug("{:#?}", self.registers())]
    registers: Cell<*mut RegisterGroup>,
}

impl CPUTrait for CPU {
    fn arc_any(self: Arc<Self>) -> Arc<dyn Any + Send + Sync> {
        self
    }
    fn id(&self) -> u64 {
        self.id
    }
    fn run(
        self: Arc<Self>,
        entry_assem_name: StringName,
        entry_type_name: StringName,
        arguments: Vec<String>,
    ) -> Result<u64> {
        let entry_type = self.vm.clone().get_type(&StringTypeReference::Single {
            assem: entry_assem_name,
            ty: entry_type_name,
        })?;
        match entry_type {
            TypeHandle::Class(entry_class) => {
                let entry_point = entry_class
                    .mt()
                    .get_method(&StringMethodReference::Single(ENTRY_SIGN))?;
                let mut args = Array::alloc_with_capacity(
                    self.clone(),
                    self.vm
                        .get_core_single_type(string_name!("System.String"))?,
                    arguments.len(),
                );
                let (arg_arr,) = args.unwrap_array_mut()?;

                for a in arguments.iter() {
                    let s = StringValue::new(a.clone());
                    let gc_ref = Gc::new(ByRefValue::String(s));
                    arg_arr.push(Value::Reference(gc_ref.clone()));
                    gc_ref.unroot();
                }
                let mut val = entry_point.call(
                    self.clone(),
                    &mut Value::Void,
                    &mut [Value::Reference(args)],
                )?;
                loop {
                    break match val {
                        Value::Void => Ok(0),
                        Value::UInt64(ret_val) => Ok(ret_val),
                        Value::RegisterReference(r) => {
                            val = self.read_register(r)?;
                            continue;
                        }
                        _ => Ok(0),
                    };
                }
            }
            _ => Err(RuntimeError::UnsupportedEntryType.into()),
        }
    }
}

impl CPU {
    pub fn from_dyn(a: Arc<dyn CPUTrait>) -> Arc<Self> {
        let a = a.arc_any();
        unsafe { a.downcast_unchecked() }
    }
    pub fn new(vm: Arc<VM>, id: u64) -> Arc<Self> {
        let this = Arc::new(Self {
            config: vm.config.read().unwrap().default_cpu_config().clone(),
            registers: Cell::new(ptr::null_mut()),
            vm: vm.clone(),
            id,
        });
        let register_group = Box::leak(Box::new(RegisterGroup::new(this.clone()))) as *mut _;
        this.registers.set(register_group);
        this
    }
}

impl CPU {
    pub fn read_register(&self, addr: u64) -> Result<Value> {
        self.registers().read(addr)
    }
    pub fn write_register(&self, addr: u64, val: Value) -> Result<()> {
        self.registers().write(addr, val)
    }
    pub fn find_register_continuous_start(&self, length: usize) -> usize {
        self.registers().find_continuous_empty_start(length)
    }
}

impl CPU {
    #[inline]
    pub fn heap_alloc<T: Trace>(&self, val: T) -> Gc<T> {
        let p = Gc::new(val);
        p.unroot();
        p
    }
}

const ENTRY_SIGN: StringName = string_name!("Main([!]System.Array`1[@T:[!]System.String])");

impl CPU {
    pub fn call_instance_method<T: GetTypeVars + GetTypeName + GetAssemblyMust + Any>(
        self: Arc<Self>,
        caller_method: Option<&CommonMethod<T>>,
        method_ref: &StringMethodReference,
        this_val: &mut Value,
        arguments: &mut [Value],
    ) -> Result<Value> {
        let ty = if let Some(caller_method) = caller_method {
            caller_method.solve_str_type(&this_val.string_type_reference())?
        } else {
            this_val.ty(self.clone())?
        };
        match ty {
            TypeHandle::Generic(_) => Err(RuntimeError::UnsupportedInstanceType.into()),
            TypeHandle::Class(class) => {
                class
                    .mt()
                    .get_method(method_ref)?
                    .call(self.clone(), this_val, arguments)
            }
            TypeHandle::Struct(s) => {
                s.mt()
                    .get_method(method_ref)?
                    .call(self.clone(), this_val, arguments)
            }
            TypeHandle::Unloaded(_) => unreachable!(),
        }
    }
    pub fn call_static_str_method(
        self: &Arc<Self>,
        type_ref: &StringTypeReference,
        method_ref: &StringMethodReference,
        args: &mut [Value],
    ) -> Result<Value> {
        match self.vm.get_type(type_ref)? {
            TypeHandle::Class(class) => {
                class
                    .mt()
                    .get_method(method_ref)?
                    .call(self.clone(), &mut Value::Void, args)
            }
            TypeHandle::Struct(s) => {
                s.mt()
                    .get_method(method_ref)?
                    .call(self.clone(), &mut Value::Void, args)
            }
            TypeHandle::Generic(_string_name) => todo!(),
            TypeHandle::Unloaded(_) => unreachable!(),
        }
    }

    pub fn create_object<T: GetTypeVars + GetTypeName + GetAssemblyMust + Any>(
        self: Arc<Self>,
        caller_method: Option<&CommonMethod<T>>,
        type_ref: &StringTypeReference,
        ctor_name: StringName,
        args: &mut [Value],
    ) -> Result<Gc<ByRefValue>> {
        let ty = if let Some(caller_method) = caller_method {
            caller_method.solve_str_type(type_ref)?
        } else {
            self.vm().get_type(type_ref)?
        };
        let class = match ty {
            TypeHandle::Class(class) => class,
            _ => return Err(RuntimeError::UnsupportedObjectType.into()),
        };
        let obj = Object::alloc(self.clone(), class.mt.get().cast());
        let mut obj_val = Value::Reference(obj.clone());
        if self
            .call_instance_method(
                caller_method,
                &StringMethodReference::Single(ctor_name),
                &mut obj_val,
                args,
            )?
            .ne(&Value::Void)
        {
            Err(RuntimeError::MethodReturnsAbnormally.into())
        } else {
            Ok(obj)
        }
    }
}

impl CPU {
    pub fn vm(&self) -> Arc<VM> {
        self.vm.clone()
    }
}

impl Drop for CPU {
    fn drop(&mut self) {
        if !self.registers.get().is_null() {
            unsafe { self.registers.get().drop_in_place() };
            self.registers.set(ptr::null_mut());
        }
    }
}

#[inline_all]
impl CPU {
    pub fn id(&self) -> u64 {
        self.id
    }
    pub(crate) fn registers(&self) -> RegisterGroup {
        unsafe { (*self.registers.get()).clone() }
    }
    pub fn register_len(&self) -> usize {
        self.registers().len()
    }
}
