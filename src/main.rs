fn main() -> std::io::Result<()> {
    grove::interface::cli::run(std::env::args().skip(1))
}
