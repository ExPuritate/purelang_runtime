use std::sync::{Arc, RwLock};

use enumflags2::{BitFlags, bitflags, make_bitflags};
use global::{
    Result, ThreadSafe, errors::RuntimeError, find_util::FindContinuousEmptyStart, inline_all,
};

use crate::value::Value;

use super::CPU;

#[derive(Clone, derive_more::Debug)]
pub struct RegisterGroup {
    #[debug(skip)]
    #[allow(unused)]
    cpu: Arc<CPU>,
    registers: Arc<RwLock<Vec<Register>>>,
}

impl RegisterGroup {
    pub fn new(cpu: Arc<CPU>) -> Self {
        let v = vec![Register::EMPTY; cpu.config.default_register_num() as usize];
        Self {
            cpu,
            registers: Arc::new(RwLock::new(v)),
        }
    }
}

#[inline_all]
impl RegisterGroup {
    pub fn read(&self, addr: u64) -> Result<Value> {
        self.registers
            .read()
            .unwrap()
            .get(addr as usize)
            .ok_or(RuntimeError::FailedGetRegister)?
            .read()
    }
    pub fn write(&self, addr: u64, val: Value) -> Result<()> {
        let mut registers = self.registers.write().unwrap();
        if registers.len() < addr as usize {
            registers.resize_with(addr as usize, Default::default);
        }
        registers
            .get_mut(addr as usize)
            .ok_or(RuntimeError::FailedGetRegister)?
            .write(val)
    }
}

impl RegisterGroup {
    pub fn find_continuous_empty_start(&self, length: usize) -> usize {
        Vec::find_continuous_empty_start(
            &mut *self.registers.write().unwrap(),
            Box::new(|r| {
                (r.val == Value::Void)
                    && r.flags.contains(RegisterFlags::Readable)
                    && r.flags.contains(RegisterFlags::Writeable)
            }),
            Box::new(Default::default),
            length,
        )
    }
}

impl RegisterGroup {
    pub fn len(&self) -> usize {
        self.registers.read().unwrap().len()
    }
}

#[bitflags]
#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Eq, Debug, ThreadSafe)]
enum RegisterFlags {
    Readable,
    Writeable,
}

#[derive(Clone, Debug, ThreadSafe)]
struct Register {
    val: Value,
    flags: BitFlags<RegisterFlags>,
}

impl Default for Register {
    fn default() -> Self {
        Self::EMPTY
    }
}

impl Register {
    const EMPTY: Self = Self {
        val: Value::Void,
        flags: make_bitflags!(RegisterFlags::{Readable | Writeable}),
    };
}

impl Register {
    fn read(&self) -> Result<Value> {
        if self.flags.contains(RegisterFlags::Readable) {
            Ok(self.val.clone())
        } else {
            Err(RuntimeError::FailedReadRegister.throw().into())
        }
    }
    fn write(&mut self, val: Value) -> Result<()> {
        if self.flags.contains(RegisterFlags::Writeable) {
            self.val = val;
            Ok(())
        } else {
            Err(RuntimeError::FailedWriteRegister.throw().into())
        }
    }
}
