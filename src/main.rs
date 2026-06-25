mod banner;
mod cli;
mod constants;
mod process;
mod prompt;
mod spec;
mod storage;

use cli::Config;
use process::ShellRunner;
use spec::Spec;

fn main() {
    banner::print();
    if let Err(err) = run() {
        eprintln!("proxmox-znc: {err}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let mut cfg = Config::from_env_and_args()?;
    let runner = ShellRunner;
    cfg.prompt_missing(&runner)?;

    let mut spec = Spec::from(&cfg);

    if cfg.dry_run {
        spec.print();
        return Ok(());
    }

    spec.validate_host(&runner)?;
    spec.install(&runner)?;
    spec.print_done()?;

    Ok(())
}
