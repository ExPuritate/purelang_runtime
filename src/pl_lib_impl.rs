#![allow(nonstandard_style)]

use crate::type_system::{Assembly, AssemblyManager};
use global::{StringName, StringTypeReference};
use std::sync::Arc;

pub mod System_Console_;
pub mod System_String;

pub mod System_Array_1;

pub mod System_Null;

pub mod System_Object;

pub mod System_ValueType;

pub mod System_Void;

pub mod System_Enum;

pub mod System_Integers;

pub mod System_Boolean;

#[cfg(test)]
mod tests;

pub trait ClassLoadToCore {
    const STRING_TYPE_REFERENCE: StringTypeReference;
    fn load_class(core_assembly: &Arc<Assembly>, assembly_manager: &AssemblyManager);
    #[allow(unused)]
    fn unwrap_single_name_of_str_type_ref() -> StringName {
        Self::STRING_TYPE_REFERENCE.unwrap_single_name_ref().clone()
    }
}

pub trait StructLoadToCore {
    const STRING_TYPE_REFERENCE: StringTypeReference;
    fn load_struct(core_assembly: &Arc<Assembly>, assembly_manager: &AssemblyManager);
    #[allow(unused)]
    fn unwrap_single_name_of_str_type_ref() -> StringName {
        Self::STRING_TYPE_REFERENCE.unwrap_single_name_ref().clone()
    }
}
