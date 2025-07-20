use export::AssemblyManagerTrait;
use global::{Result, StringName, StringTypeReference, indexmap, string_name};
use std::sync::Arc;

use super::AssemblyManager;

#[test]
fn test_generic() -> Result<()> {
    let assembly_manager = AssemblyManager::new()?;
    let object_type = assembly_manager.get_type_from_str(&StringTypeReference::Single {
        assem: string_name!("!"),
        ty: string_name!("System.Object"),
    })?;
    let array_type = assembly_manager.get_type_from_str(&StringTypeReference::Single {
        assem: string_name!("!"),
        ty: string_name!("System.Array`1"),
    })?;
    let generated_array_type = array_type.make_generic(Arc::new(indexmap! {
        StringName::from_static_str("T") => object_type
    }))?;
    dbg!(generated_array_type.unwrap_class_ref());
    dbg!(generated_array_type.string_reference());
    dbg!(generated_array_type.type_vars());
    Ok(())
}

#[test]
fn test_parse_binary() -> global::Result<()> {
    let assembly_manager = AssemblyManager::new()?;
    assembly_manager
        .clone()
        .load_from_binary_assemblies(&[dbg!(binary::Assembly::from_file(
            r"..\binary\test.plb"
        )?)])?;
    dbg!(assembly_manager.all_types()[&string_name!("Test.Test")].keys());
    Ok(())
}
