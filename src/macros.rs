/// 不换行打印并立即刷新 stdout，用回车覆盖当前行
/// 用于终端内联状态更新（tracing 不擅长这个场景）
macro_rules! printfl_inline {
    ($($arg:tt)*) => {{
        print!("\r");
        print!($($arg)*);
        let _ = std::io::Write::flush(&mut std::io::stdout());
    }};
}

pub(crate) use printfl_inline;
