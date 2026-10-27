fn main() -> std::process::ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let cwd = match std::env::current_dir() {
        Ok(cwd) => cwd,
        Err(err) => {
            eprintln!("reading the working directory: {err}");
            return std::process::ExitCode::from(2);
        }
    };
    let rendered = slopmeter::cli::run(&cwd, &args);
    print!("{}", rendered.stdout);
    eprint!("{}", rendered.stderr);
    std::process::ExitCode::from(rendered.code as u8)
}
