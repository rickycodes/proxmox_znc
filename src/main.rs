mod cli;
mod constants;
mod process;
mod prompt;
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
    const PINK: &str = "\x1b[38;5;205m";
    const CYAN: &str = "\x1b[38;5;51m";
    const RESET: &str = "\x1b[0m";

    println!(
        "\n {pink}╔════════════════════════════╗{reset}\n\
{pink} ║ {cyan}███████{pink}╗{cyan}███{pink}╗   {cyan}██{pink}╗{cyan}███████{pink}╗ ║{reset}\n\
{pink} ║ ╚══{cyan}███{pink}╔╝{cyan}████{pink}╗  {cyan}██{pink}║{cyan}██{pink}╔════╝ ║{reset}\n\
{pink} ║ {cyan}  ███{pink}╔╝ {cyan}██{pink}╔{cyan}██{pink}╗ {cyan}██{pink}║{cyan}██{pink}║      ║{reset}\n\
{pink} ║ {cyan} ███{pink}╔╝  {cyan}██{pink}║╚{cyan}██{pink}╗{cyan}██{pink}║{cyan}██{pink}║      ║{reset}\n\
{pink} ║ {cyan}███████{pink}╗{cyan}██{pink}║ {pink}╚{cyan}████{pink}║{cyan}███████{pink}╗ ║{reset}\n\
{pink} ║ ╚══════╝╚═╝  ╚═══╝╚══════╝ ║{reset}\n\
{pink} ╚════════════════════════════╝{reset}\n\
\n\
{cyan}Proxmox ZNC installer{reset}\n",
        pink = PINK,
        cyan = CYAN,
        reset = RESET
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
