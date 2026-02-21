fn main() -> std::io::Result<()> {
    grove::interface::daemon::run(std::env::args().skip(1))
}
