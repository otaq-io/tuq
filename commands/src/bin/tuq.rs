fn main() {
    std::process::exit(tuq_commands::tuq::run(
        std::env::args_os().skip(1).collect(),
    ));
}
