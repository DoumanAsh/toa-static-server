#[macro_export]
macro_rules! error_println {
    ($($tt:tt)*) => {{
        use ::std::io::Write;
        let _ = writeln!(&mut ::std::io::stderr(), $($tt)*);
    }}
}
