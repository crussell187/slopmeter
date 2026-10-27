fn main() -> std::process::ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let Some(rendered) = slopmeter::cli::run(&args) else {
        return std::process::ExitCode::SUCCESS;
    };
    print!("{}", rendered.stdout);
    eprint!("{}", rendered.stderr);
    std::process::ExitCode::from(rendered.code as u8)
}
