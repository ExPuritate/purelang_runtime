use crate::pl_lib_impl::System_Console_;

#[test]
fn test_colors() -> global::Result<()> {
    let b_color = System_Console_::background_color()?;
    let f_color = System_Console_::foreground_color()?;
    dbg!(b_color);
    dbg!(f_color);
    Ok(())
}
