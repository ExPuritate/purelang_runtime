#![feature(tuple_trait)]
#![feature(unboxed_closures)]
#![feature(fn_traits)]
#![feature(iterator_try_collect)]
#![feature(ptr_as_ref_unchecked)]
#![feature(ptr_as_uninit)]
#![feature(formatting_options)]
#![feature(decl_macro)]
#![feature(macro_metavar_expr)]
#![feature(likely_unlikely)]
#![feature(format_args_nl)]
#![feature(once_cell_get_mut)]
#![feature(lock_value_accessors)]
#![feature(stmt_expr_attributes)]
#![feature(generic_atomic)]
#![feature(downcast_unchecked)]
#![allow(
    clippy::not_unsafe_ptr_arg_deref,
    clippy::type_complexity,
    non_local_definitions,
    static_mut_refs,
    clippy::missing_transmute_annotations,
    non_snake_case
)]

mod pl_lib_impl;
pub mod type_system;
pub mod value;
pub mod vm;
