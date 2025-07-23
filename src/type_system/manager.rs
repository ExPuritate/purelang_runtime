use super::{
    Assembly, Class, ClassField, CommonMethod, CommonMethodTable, GenericBinding, TypeHandle,
    TypeVar,
};
use crate::pl_lib_impl::System_Array_1::System_Array;
use crate::pl_lib_impl::System_Boolean::System_Boolean;
use crate::pl_lib_impl::System_Console_::to_vm::System_Console;
use crate::pl_lib_impl::System_Console_::to_vm::System_ConsoleColor;
use crate::pl_lib_impl::System_Enum::System_Enum;
use crate::pl_lib_impl::System_Null::System_Null;
use crate::pl_lib_impl::System_Object::System_Object;
use crate::pl_lib_impl::System_String::System_String;
use crate::pl_lib_impl::System_ValueType::System_ValueType;
use crate::pl_lib_impl::System_Void::System_Void;
use crate::pl_lib_impl::{ClassLoadToCore, StructLoadToCore, System_Integers};
use crate::type_system::Struct;
use crate::type_system::StructField;
use binary::TypeDef;
use export::{AssemblyManagerTrait, AssemblyTrait};
use global::{IndexMap, Result, StringName, StringTypeReference, ThreadSafe, errors::RuntimeError};
use std::any::Any;
use std::hint::unreachable_unchecked;
use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

#[derive(derive_more::Debug, Clone, ThreadSafe)]
pub struct AssemblyManager {
    #[debug("{:#?}", assemblies.read().unwrap().keys().map(|x| &**x).collect::<Vec<_>>())]
    assemblies: Arc<RwLock<HashMap<StringName, Arc<Assembly>>>>,
}

#[allow(non_upper_case_globals)]
impl AssemblyManager {
    pub const System_ValueType_STRUCT_REF: StringTypeReference =
        StringTypeReference::core_static_single_type("System.ValueType");
    pub const System_Void_STRUCT_REF: StringTypeReference =
        StringTypeReference::core_static_single_type("System.Void");
    pub const System_Object_CLASS_REF: StringTypeReference =
        StringTypeReference::core_static_single_type("System.Object");
    pub const System_Null_CLASS_REF: StringTypeReference =
        StringTypeReference::core_static_single_type("System.Null");
}

impl AssemblyManager {
    pub fn new() -> Result<Arc<Self>> {
        let this = Arc::new(Self {
            assemblies: Default::default(),
        });
        this.clone().load_core()?;
        Ok(this)
    }
    pub fn from_dyn(d: Arc<dyn AssemblyManagerTrait>) -> Arc<Self> {
        unsafe { d.arc_any().downcast_unchecked() }
    }
}

impl AssemblyManager {
    pub fn get_type_from_str_complex(
        &self,
        type_vars_lookup: &dyn Fn(&StringName) -> Option<TypeHandle>,
        type_ref: &StringTypeReference,
    ) -> Result<TypeHandle> {
        match type_ref {
            StringTypeReference::Single { assem, ty } => {
                Ok(Assembly::from_dyn(self.get_assembly(assem.clone())?)
                    .get_single_type(ty.clone())?)
            }
            StringTypeReference::Generic(g) => {
                if let Some(g) = type_vars_lookup(g) {
                    Ok(g)
                } else {
                    Err(RuntimeError::FailedGetType(type_ref.clone()).into())
                }
            }
            StringTypeReference::WithGeneric {
                assem,
                ty,
                type_vars,
            } => {
                let handle = Assembly::from_dyn(self.get_assembly(assem.clone())?)
                    .get_single_type(ty.clone())?;
                let type_vars = Arc::new(
                    type_vars
                        .iter()
                        .map(|(k, v)| {
                            Ok::<_, global::Error>((
                                k.clone(),
                                self.get_type_from_str_complex(type_vars_lookup, v)?,
                            ))
                        })
                        .try_collect::<IndexMap<_, _>>()?,
                );
                handle.make_generic(type_vars)
            }
        }
    }
    pub fn get_type_from_str(&self, type_ref: &StringTypeReference) -> Result<TypeHandle> {
        self.get_type_from_str_complex(&|_| None, type_ref)
    }
    pub fn all_types(&self) -> HashMap<StringName, HashMap<StringName, TypeHandle>> {
        let assemblies = self.assemblies.read().unwrap();
        let mut map = HashMap::with_capacity(assemblies.len());
        for (assem_name, assem) in assemblies.iter() {
            let types = assem.types().read().unwrap();
            map.insert(assem_name.clone(), types.clone());
        }
        map
    }
}

impl AssemblyManagerTrait for AssemblyManager {
    fn arc_any(self: Arc<Self>) -> Arc<dyn Any + Send + Sync> {
        self
    }

    fn load_from_binary_assemblies(
        self: Arc<Self>,
        binary_assemblies: &[binary::Assembly],
    ) -> Result<()> {
        let this = &self;
        fn map_ty(s: StringTypeReference) -> TypeHandle {
            match s {
                StringTypeReference::Generic(g) => TypeHandle::Generic(g),
                x => TypeHandle::Unloaded(x),
            }
        }
        for b_assem in binary_assemblies {
            let assembly = Arc::new(Assembly::new(b_assem.name().clone(), this));
            let types = b_assem.type_defs();
            for ty in types.values() {
                let ty = match ty {
                    TypeDef::Class(c) => {
                        TypeHandle::Class(Class::try_new(
                            &assembly,
                            c.attr(),
                            c.name().clone(),
                            |class| {
                                CommonMethodTable::try_new(
                                    |mt_ptr| {
                                        c.methods().iter().map(|(m_name, m)| {
                                            let method = CommonMethod::new(
                                                m.name().clone(),
                                                m.attr(),
                                                mt_ptr,
                                                (&**m.instructions()).into(),
                                                map_ty(m.ret_type().clone()),
                                                m.args()
                                                    .iter()
                                                    .cloned()
                                                    .map(map_ty)
                                                    .collect(),
                                                Arc::new(m.type_vars().iter()
                                                    .map(|(g_name, g_binding)| (g_name.clone(), TypeVar::Canon(GenericBinding {
                                                        implemented_interfaces: g_binding
                                                            .implemented_interfaces()
                                                            .iter()
                                                            .cloned()
                                                            .map(map_ty)
                                                            .collect(),
                                                        parent: g_binding.parent().clone().map(TypeHandle::Unloaded),
                                                    }))).collect()),
                                            );
                                            Ok((m_name.clone(), method))
                                        }).try_collect()
                                    },
                                    &class,
                                    c.parent().clone().map(TypeHandle::Unloaded),
                                )
                            },
                            c.fields()
                                .iter()
                                .map(|(f_name, f)| {
                                    Ok::<_, global::Error>((
                                        f_name.clone(),
                                        ClassField::new(
                                            f.name().clone(),
                                            f.attr(),
                                            map_ty(f.ty().clone()),
                                        ),
                                    ))
                                })
                                .try_collect()?,
                        )?)
                    }
                    TypeDef::Struct(s) => {
                        TypeHandle::Struct(Struct::try_new(
                            &assembly,
                            s.attr(),
                            s.name().clone(),
                            |r#struct| {
                                CommonMethodTable::try_new(
                                    |mt_ptr| {
                                        s.methods().iter().map(|(m_name, m)| {
                                            let method = CommonMethod::new(
                                                m.name().clone(),
                                                m.attr(),
                                                mt_ptr,
                                                (&**m.instructions()).into(),
                                                map_ty(m.ret_type().clone()),
                                                m.args()
                                                    .iter()
                                                    .cloned()
                                                    .map(map_ty)
                                                    .collect(),
                                                Arc::new(m.type_vars().iter()
                                                    .map(|(g_name, g_binding)| (g_name.clone(), TypeVar::Canon(GenericBinding {
                                                        implemented_interfaces: g_binding
                                                            .implemented_interfaces()
                                                            .iter()
                                                            .cloned()
                                                            .map(map_ty)
                                                            .collect(),
                                                        parent: g_binding.parent().clone().map(TypeHandle::Unloaded),
                                                    }))).collect()),
                                            );
                                            Ok((m_name.clone(), method))
                                        }).try_collect()
                                    },
                                    &r#struct,
                                    s.parent().clone().map(TypeHandle::Unloaded),
                                )
                            },
                            s.fields()
                                .iter()
                                .map(|(f_name, f)| {
                                    Ok::<_, global::Error>((
                                        f_name.clone(),
                                        StructField::new(
                                            f.name().clone(),
                                            f.attr(),
                                            map_ty(f.ty().clone()),
                                        ),
                                    ))
                                })
                                .try_collect()?,
                        )?)
                    }
                };
                assembly.add_type(ty);
            }
            self.add_assembly(assembly);
        }
        self.resolve_type_references()?;
        Ok(())
    }
    fn get_assembly(&self, assem_name: StringName) -> Result<Arc<dyn AssemblyTrait>> {
        self.assemblies
            .read()
            .unwrap()
            .get(&assem_name)
            .ok_or(RuntimeError::FailedGetAssembly.throw().into())
            .map(|x| {
                let x: Arc<dyn AssemblyTrait> = x.clone();
                x
            })
    }
    fn add_assembly(&self, assem: Arc<dyn AssemblyTrait>) {
        let mut assemblies = self.assemblies.write().unwrap();
        assemblies.insert(assem.name(), Assembly::from_dyn(assem));
    }
    fn load_core(self: Arc<Self>) -> Result<()> {
        let core_assembly = Arc::new(Assembly::new(
            StringTypeReference::CORE_ASSEMBLY_NAME,
            &self,
        ));

        self.add_assembly(core_assembly.clone());
        //<editor-fold desc="Basic Types">
        System_Object::load_class(&core_assembly, &self);
        System_ValueType::load_struct(&core_assembly, &self);
        //</editor-fold>

        System_Void::load_struct(&core_assembly, &self);
        System_Boolean::load_struct(&core_assembly, &self);
        System_Enum::load_struct(&core_assembly, &self);

        //<editor-fold desc="Enums">
        System_ConsoleColor::load_struct(&core_assembly, &self);
        //</editor-fold>

        System_Integers::load_integers(&core_assembly, &self);
        System_Null::load_class(&core_assembly, &self);
        System_Array::load_class(&core_assembly, &self);
        System_String::load_class(&core_assembly, &self);
        System_Console::load_class(&core_assembly, &self);
        self.add_assembly(core_assembly);
        self.resolve_type_references()?;
        Ok(())
    }

    fn resolve_type_references(&self) -> Result<()> {
        fn map_type_vars(
            assembly_manager: &AssemblyManager,
            type_vars_origin: &Arc<IndexMap<StringName, TypeVar>>,
        ) -> Result<IndexMap<StringName, TypeVar>> {
            let mut type_vars = IndexMap::new();
            for (t_name, type_var) in type_vars_origin.iter() {
                if let TypeVar::Type(TypeHandle::Unloaded(t)) = type_var {
                    type_vars.insert(
                        t_name.clone(),
                        TypeVar::Type(assembly_manager.get_type_from_str(t)?),
                    );
                } else if let TypeVar::Canon(GenericBinding {
                    implemented_interfaces,
                    parent,
                }) = type_var
                {
                    let parent = if let Some(parent) = parent {
                        if let TypeHandle::Unloaded(p) = parent {
                            Some(assembly_manager.get_type_from_str(p)?)
                        } else {
                            Some(parent.clone())
                        }
                    } else {
                        None
                    };
                    let mut operated_implemented_interfaces = Vec::new();
                    for i in implemented_interfaces {
                        if let TypeHandle::Unloaded(s) = i {
                            operated_implemented_interfaces
                                .push(assembly_manager.get_type_from_str(s)?);
                        } else {
                            operated_implemented_interfaces.push(i.clone());
                        }
                    }
                    type_vars.insert(
                        t_name.clone(),
                        TypeVar::Canon(GenericBinding {
                            parent,
                            implemented_interfaces: operated_implemented_interfaces,
                        }),
                    );
                } else {
                    type_vars.insert(t_name.clone(), type_var.clone());
                }
            }
            Ok(type_vars)
        }
        fn map_type_handle(this: &AssemblyManager, handle: &mut TypeHandle) -> global::Result<()> {
            eprintln!("Resolving TypeHandle: {}", handle.name());
            if let TypeHandle::Unloaded(r) = handle {
                let t = this.get_type_from_str(r)?;
                *handle = t;
            }
            Ok(())
        }
        for assembly in self.assemblies.read().unwrap().clone().into_values() {
            eprintln!("Resolving {}", assembly.name());
            let types = assembly.types().read().unwrap();
            for (ty_name, ty) in types.iter() {
                eprintln!("Resolving {ty_name}");
                let mut ty = ty.clone();
                map_type_handle(self, &mut ty)?;
                match ty {
                    TypeHandle::Class(ref c) => {
                        let mt = unsafe { &mut *c.mt.get() };
                        for method in mt.map.values_mut() {
                            method.type_vars = Arc::new(map_type_vars(self, &method.type_vars)?);
                            for arg_type in method.args.iter_mut() {
                                if let TypeHandle::Unloaded(r) = arg_type {
                                    let t = self.get_type_from_str(r)?;
                                    *arg_type = t;
                                }
                            }
                        }
                        if let Some(ref mut parent) = mt.parent {
                            map_type_handle(self, parent)?;
                        }
                    }
                    TypeHandle::Struct(ref s) => {
                        let mt = unsafe { &mut *s.mt.get() };
                        for method in mt.map.values_mut() {
                            method.type_vars = Arc::new(map_type_vars(self, &method.type_vars)?);
                            for arg_type in method.args.iter_mut() {
                                if let TypeHandle::Unloaded(r) = arg_type {
                                    let t = self.get_type_from_str(r)?;
                                    *arg_type = t;
                                }
                            }
                        }
                        if let Some(ref mut parent) = mt.parent {
                            map_type_handle(self, parent)?;
                        }
                    }
                    TypeHandle::Generic(_) => {}
                    TypeHandle::Unloaded(_) => unsafe { unreachable_unchecked() },
                }
            }
        }
        Ok(())
    }
}

#[unsafe(no_mangle)]
#[allow(nonstandard_style)]
pub extern "Rust" fn NewAssemblyManager() -> global::Result<Arc<dyn AssemblyManagerTrait>> {
    let assembly_manager: Arc<dyn AssemblyManagerTrait> = AssemblyManager::new()?;
    Ok(assembly_manager)
}
