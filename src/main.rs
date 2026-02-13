fn main() -> std::io::Result<()> {
    if std::env::args().any(|arg| arg == "--print-hello") {
        println!("{}", grove::hello_message("grove"));
        return Ok(());
    }

    grove::run_tui()
}
