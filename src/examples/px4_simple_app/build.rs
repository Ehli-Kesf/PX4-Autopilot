macro_rules! p {
    ($($tokens: tt)*) => {
        println!("cargo:warning={}", format!($($tokens)*))
    }
}
fn main() {
    p!("olaaaaaaaaaaaaaaa");
    let var = "CMAKE_BINARY_DIR";
    let platform = std::env::var(var).unwrap_or("OKUNAMADI!!!".into());
    if let Some(hello) = std::env::var("DENEME") {
        p!("DENEME: {hello}");
    }
    p!("hello rust matek build system with cmake var {var}: {platform}");
}
