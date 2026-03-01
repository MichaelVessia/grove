fn main() -> std::io::Result<()> {
    grove::cli::run(std::env::args().skip(1))
}
