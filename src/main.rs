mod cli;
mod constants;
mod process;
mod prompt;
mod storage;
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
        pink = "\x1b[38;5;205m",
        cyan = "\x1b[38;5;51m",
        reset = "\x1b[0m"
    );
}

fn run() -> Result<(), String> {
    let mut cfg = Config::from_env_and_args()?;
    let runner = ShellRunner;
    cfg.prompt_missing(&runner)?;

    let spec = Spec::from(&cfg);

    if cfg.dry_run {
        spec.print();
        return Ok(());
    }

    spec.validate_host(&runner)?;
    spec.install(&runner)?;
    spec.print_done(&runner)?;

    Ok(())
}
