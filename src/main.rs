mod cli;
mod prompt;
mod process;
mod spec;

use cli::Config;
use process::ShellRunner;
use spec::Spec;

fn main() {
    banner();
    if let Err(err) = run() {
        eprintln!("proxmox-znc: {err}");
        std::process::exit(1);
    }
}

fn banner() {
    println!(
        r#"
 ███████╗███╗   ██╗███████╗
 ╚══███╔╝████╗  ██║██╔════╝
   ███╔╝ ██╔██╗ ██║██║  
  ███╔╝  ██║╚██╗██║██║  
 ███████╗██║ ╚████║███████╗
 ╚══════╝╚═╝  ╚═══╝╚══════╝

Proxmox ZNC installer
"#
    );
}

fn run() -> Result<(), String> {
    let mut cfg = Config::from_env_and_args()?;
    cfg.prompt_missing()?;

    let spec = Spec::from(&cfg);

    if cfg.dry_run {
        spec.print();
        return Ok(());
    }

    let runner = ShellRunner;
    spec.validate_host(&runner)?;
    spec.install(&runner)?;
    spec.print_done(&runner)?;

    Ok(())
}
