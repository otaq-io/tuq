fn main() {
    std::process::exit(tuq_commands::plan::run(
        std::env::args_os().skip(1).collect(),
    ));
}
