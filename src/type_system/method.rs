use super::{AssemblyManager, CommonMethodTable, TypeHandle, TypeVar, get_traits::GetTypeName};
use crate::type_system::get_traits::{GetAssemblyMust, GetInstruction, GetTypeVars};
use crate::value::{Array, ByRefValue};
use crate::{value::Value, vm::CPU};
use export::AssemblyTrait;
use global::StringMethodReference;
use global::instruction::StringInstruction;
use global::{
    IndexMap, Result, StringName, StringTypeReference, attrs::MethodAttr,
    errors::RuntimeError, string_name,
};
use std::{any::Any, cell::Cell, sync::Arc};

#[allow(clippy::type_complexity, unused)]
#[derive(derive_more::Debug)]
pub struct CommonMethod<T: Any + GetTypeName> {
    #[debug(skip)]
    pub(crate) mt: Cell<*mut CommonMethodTable<T>>,
    #[debug("<EntryPoint>")]
    pub(crate) entry_point:
        Arc<dyn Fn(&Self, Arc<CPU>, &mut Value, &mut [Value], u64) -> Result<Value>>,
    pub(crate) name: StringName,
    pub(crate) attr: MethodAttr,
    pub(crate) instructions: Arc<[StringInstruction]>,
    #[debug("{:#?}", ret_type.name())]
    pub(crate) ret_type: TypeHandle,
    #[debug("{:#?}", args.iter().map(|x| x.name()).collect::<Vec<_>>())]
    pub(crate) args: Vec<TypeHandle>,
    #[debug("{:#?}", type_vars.iter().map(|x| (x.0, x.1.name())).collect::<IndexMap<_, _>>())]
    pub(crate) type_vars: Arc<IndexMap<StringName, TypeVar>>,
}

impl<T: Any + GetTypeName> Clone for CommonMethod<T> {
    fn clone(&self) -> Self {
        let ptr = self.mt.get() as usize;
        Self {
            name: self.name.clone(),
            attr: self.attr,
            mt: Cell::new(ptr as *mut CommonMethodTable<T>),
            instructions: self.instructions.clone(),
            ret_type: self.ret_type.clone(),
            args: self.args.clone(),
            entry_point: self.entry_point.clone(),
            type_vars: self.type_vars.clone(),
        }
    }
}

impl<T: Any + GetTypeName + GetAssemblyMust + GetTypeVars> CommonMethod<T> {
    pub fn new(
        name: StringName,
        attr: MethodAttr,
        mt: *mut CommonMethodTable<T>,
        instructions: Arc<[StringInstruction]>,
        ret_type: TypeHandle,
        args: Vec<TypeHandle>,
        type_vars: Arc<IndexMap<StringName, TypeVar>>,
    ) -> Self {
        Self {
            name,
            attr,
            mt: Cell::new(mt),
            instructions,
            ret_type,
            args,
            entry_point: Arc::new(default_entry_point),
            type_vars,
        }
    }
    pub fn native(
        name: StringName,
        attr: MethodAttr,
        mt: *mut CommonMethodTable<T>,
        ret_type: TypeHandle,
        args: Vec<TypeHandle>,
        type_vars: Arc<IndexMap<StringName, TypeVar>>,
        entry_point: impl Fn(&Self, Arc<CPU>, &mut Value, &mut [Value], u64) -> Result<Value> + 'static,
    ) -> Self {
        Self {
            name,
            attr,
            mt: Cell::new(mt),
            instructions: vec![].into(),
            ret_type,
            args,
            entry_point: Arc::new(entry_point),
            type_vars,
        }
    }
}

impl<T: Any + GetTypeName> CommonMethod<T> {
    pub fn call(&self, cpu: Arc<CPU>, this_val: &mut Value, args: &mut [Value]) -> Result<Value> {
        (self.entry_point)(
            self,
            cpu.clone(),
            this_val,
            args,
            cpu.find_register_continuous_start(self.attr.register_len() as _) as _,
        )
    }
}

impl<T: Any + GetTypeName> CommonMethod<T> {
    pub fn make_generic(&self, type_vars: Arc<IndexMap<StringName, TypeHandle>>) -> Result<Self> {
        let name = StringMethodReference::WithGeneric(
            self.name.clone(),
            Arc::new(
                type_vars
                    .iter()
                    .map(|(k, v)| (k.clone(), v.string_reference()))
                    .collect::<IndexMap<_, _>>(),
            ),
        )
        .string_name_repr();
        Ok(Self {
            name,
            attr: self.attr,
            mt: self.mt.clone(),
            instructions: self.instructions.clone(),
            ret_type: self.ret_type.clone(),
            args: self.args.clone(),
            entry_point: self.entry_point.clone(),
            type_vars: Arc::new(
                type_vars
                    .iter()
                    .map(|(k, v)| (k.clone(), TypeVar::Type(v.clone())))
                    .collect::<IndexMap<_, _>>(),
            ),
        })
    }
    pub fn mt(&self) -> &CommonMethodTable<T> {
        unsafe { &*self.mt.get() }
    }
}

impl<T: GetTypeVars + GetTypeName + GetAssemblyMust + 'static> CommonMethod<T> {
    pub fn solve_str_type(&self, type_reference: &StringTypeReference) -> Result<TypeHandle> {
        match type_reference {
            StringTypeReference::Generic(ty) => {
                if let Some(TypeVar::Type(t)) = self.type_vars.get(ty) {
                    return Ok(t.clone());
                }
                let mt = unsafe { &*self.mt.get() };
                mt.ty()
                    .type_vars()
                    .get(ty)
                    .filter(|x| matches!(x, TypeVar::Type(_)))
                    .map(|x| {
                        let TypeVar::Type(t) = x else { unreachable!() };
                        t
                    })
                    .cloned()
                    .ok_or(RuntimeError::FailedGetType(type_reference.clone()).into())
            }
            _ => {
                let mt = unsafe { &*self.mt.get() };
                AssemblyManager::from_dyn(mt.ty().must_assembly().manager())
                    .get_type_from_str_complex(
                        &|s| {
                            if let Some(TypeVar::Type(t)) = self.type_vars.get(s) {
                                return Some(t.clone());
                            }
                            mt.ty()
                                .type_vars()
                                .get(s)
                                .and_then(|x| {
                                    let TypeVar::Type(t) = x else {
                                        return None;
                                    };
                                    Some(t)
                                })
                                .cloned()
                        },
                        type_reference,
                    )
            }
        }
    }
}

#[allow(
    clippy::match_ref_pats,
    clippy::too_many_arguments,
    clippy::only_used_in_recursion
)]
fn match_code<T: GetTypeVars + GetTypeName + GetAssemblyMust + Any>(
    #[allow(unused)] method: &CommonMethod<T>,
    #[allow(unused)] cpu: Arc<CPU>,
    #[allow(unused)] this_val: &mut Value,
    #[allow(unused)] args: &[Value],
    #[allow(unused)] register_start: u64,
    #[allow(unused)] ins: &StringInstruction,
    #[allow(unused)] pc: &mut usize,
    #[allow(unused)] res: &mut Option<Value>,
) -> Result<()> {
    match ins {
        &StringInstruction::LoadTrue { register_addr } => {
            cpu.write_register(register_start + register_addr, Value::True)?
        }
        &StringInstruction::LoadFalse { register_addr } => {
            cpu.write_register(register_start + register_addr, Value::False)?
        }
        &StringInstruction::Load_u8 { register_addr, val } => {
            cpu.write_register(register_start + register_addr, Value::UInt8(val))?
        }
        &StringInstruction::Load_u8_0 { register_addr } => {
            cpu.write_register(register_start + register_addr, Value::UInt8(0))?
        }
        &StringInstruction::Load_u8_1 { register_addr } => {
            cpu.write_register(register_start + register_addr, Value::UInt8(1))?
        }
        &StringInstruction::Load_u8_2 { register_addr } => {
            cpu.write_register(register_start + register_addr, Value::UInt8(2))?
        }
        &StringInstruction::Load_u8_3 { register_addr } => {
            cpu.write_register(register_start + register_addr, Value::UInt8(3))?
        }
        &StringInstruction::Load_u8_4 { register_addr } => {
            cpu.write_register(register_start + register_addr, Value::UInt8(4))?
        }
        &StringInstruction::Load_u8_5 { register_addr } => {
            cpu.write_register(register_start + register_addr, Value::UInt8(5))?
        }
        &StringInstruction::Load_u64 { register_addr, val } => {
            cpu.write_register(register_start + register_addr, Value::UInt64(val))?
        }
        &StringInstruction::LoadArg { register_addr, arg } => {
            cpu.write_register(register_start + register_addr, args[arg as usize].clone())?
        }
        #[allow(deprecated)]
        &StringInstruction::LoadAllArgsAsArray { register_addr } => cpu.write_register(
            register_start + register_addr,
            Value::Reference(Array::alloc_with_data(
                cpu.clone(),
                cpu.vm()
                    .get_core_single_type(string_name!("System.String"))?,
                args,
            )),
        )?,
        StringInstruction::InstanceCall {
            val,
            method: method_target,
            args,
            ret_at,
        } => {
            let mut val = cpu.read_register(*val)?;
            let res = cpu.clone().call_instance_method(
                Some(method),
                method_target,
                &mut val,
                args.iter()
                    .map(|x| cpu.read_register(*x))
                    .try_collect::<Vec<_>>()?
                    .as_mut_slice(),
            )?;
            cpu.write_register(*ret_at, res)?;
        }
        StringInstruction::StaticCall {
            ty,
            method: method_target,
            args,
            ret_at,
        } => {
            let res = cpu.call_static_str_method(
                ty,
                method_target,
                args.iter()
                    .map(|x| cpu.read_register(*x))
                    .try_collect::<Vec<_>>()?
                    .as_mut_slice(),
            )?;
            cpu.write_register(*ret_at, res)?;
        }
        StringInstruction::LoadStatic {
            register_addr,
            ty,
            name,
        } => {
            let v = cpu.vm().get_static_from_str(ty, name)?;
            cpu.write_register(*register_addr, v)?;
        }
        StringInstruction::NewObject {
            ty,
            ctor_name,
            args,
            register_addr,
        } => {
            let mut args = args
                .iter()
                .map(|x| cpu.read_register(*x))
                .try_collect::<Vec<_>>()?;
            let v = cpu
                .clone()
                .create_object(Some(method), ty, ctor_name.clone(), &mut args)?;
            cpu.write_register(*register_addr, Value::Reference(v))?;
        }
        &StringInstruction::ReturnVal { register_addr } => {
            let v = cpu.read_register(register_addr)?;
            *res = Some(v);
        }
        StringInstruction::SetField {
            register_addr,
            field,
        } => {
            let register_addr = *register_addr;
            let val = cpu.read_register(register_addr)?;
            match this_val {
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
                | Value::Int128(_) => {
                    return Err(RuntimeError::FailedGetField(field.clone()).into());
                }
                Value::Struct(s) => {
                    s.get_mut_field(field.as_str())?.set_val(&val);
                }
                Value::Reference(r) => match &mut **r {
                    ByRefValue::Object(obj) => {
                        obj.get_mut_field(field.clone())?.set_val(&val);
                    }
                    ByRefValue::Array(_) | ByRefValue::String(_) | ByRefValue::Null => {
                        return Err(RuntimeError::FailedGetField(field.clone()).into());
                    }
                },
                Value::RegisterReference(r) => {
                    let mut _this = cpu.read_register(*r)?;
                    match_code(
                        method,
                        cpu,
                        &mut _this,
                        args,
                        register_start,
                        &StringInstruction::SetField {
                            register_addr,
                            field: field.clone(),
                        },
                        pc,
                        res,
                    )?;
                    *this_val = _this;
                }
            }
        }
    }
    Ok(())
}

fn default_entry_point<T: Any + GetTypeName + GetAssemblyMust + GetTypeVars>(
    method: &CommonMethod<T>,
    cpu: Arc<CPU>,
    this_val: &mut Value,
    args: &mut [Value],
    register_start: u64,
) -> Result<Value> {
    let instructions = &method.instructions();
    let mut pc = 0usize;
    let mut res = None;
    loop {
        if pc >= instructions.len() {
            return Ok(res.unwrap_or_default());
        }
        if res.is_some() {
            unsafe {
                return Ok(res.unwrap_unchecked());
            }
        }
        let i = &instructions[pc];
        match_code(
            method,
            cpu.clone(),
            this_val,
            args,
            register_start,
            i,
            &mut pc,
            &mut res,
        )?;
        pc += 1;
    }
}
