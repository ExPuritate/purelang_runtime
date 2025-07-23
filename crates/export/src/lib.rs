#![allow(nonstandard_style)]
#![feature(trait_alias)]

pub extern crate binary;

use global::StringName;
use std::any::Any;
use std::sync::Arc;

pub trait VMTrait: VMTrait_Assembly + VMTrait_CPU + VMTrait_Statics {}

pub trait VMTrait_CPU {
    fn new_cpu(self: Arc<Self>) -> (u64, Arc<dyn CPUTrait>);
    fn get_cpu(&self, index: usize) -> Option<Arc<dyn CPUTrait>>;
}

trait AssemblyLookuper = Fn(&str) -> Option<String>;

pub trait VMTrait_Assembly {
    fn get_core_assem(&self) -> Arc<dyn AssemblyTrait>;
    fn get_assembly(&self, name: StringName) -> global::Result<Arc<dyn AssemblyTrait>>;
    fn assembly_manager(&self) -> Arc<dyn AssemblyManagerTrait>;
    fn add_assembly_lookuper(&self, f: Arc<dyn AssemblyLookuper>);
    fn add_assembly_dir(&self, p: &str);
}

pub trait VMTrait_Statics {
    fn load_statics(self: Arc<Self>) -> global::Result<()>;
}

pub trait CPUTrait {
    fn arc_any(self: Arc<Self>) -> Arc<dyn Any + Send + Sync>;
    fn id(&self) -> u64;
    fn run(
        self: Arc<Self>,
        entry_assem_name: StringName,
        entry_type_name: StringName,
        arguments: Vec<String>,
    ) -> global::Result<u64>;
}

pub trait AssemblyManagerTrait {
    fn arc_any(self: Arc<Self>) -> Arc<dyn Any + Send + Sync>;
    fn load_from_binary_assemblies(
        self: Arc<Self>,
        assemblies: &[binary::Assembly],
    ) -> global::Result<()>;
    fn get_assembly(&self, assem_name: StringName) -> global::Result<Arc<dyn AssemblyTrait>>;
    fn add_assembly(&self, assem: Arc<dyn AssemblyTrait>);
    fn load_core(self: Arc<Self>) -> global::Result<()>;
    fn resolve_type_references(&self) -> global::Result<()>;
}

pub trait AssemblyTrait {
    fn arc_any(self: Arc<Self>) -> Arc<dyn Any + Send + Sync>;
    fn name(&self) -> StringName;
    fn manager(&self) -> Arc<dyn AssemblyManagerTrait>;
}
