use crate::constants;
use std::env;

pub fn print() {
    let header = if hyperlinks_supported() {
        let user_url = format!("{}/{}", constants::GITHUB_BASE_URL, constants::GITHUB_USER);
        let repo_url = format!(
            "{}/{}/{}",
            constants::GITHUB_BASE_URL,
            constants::GITHUB_USER,
            constants::GITHUB_REPO
        );
        format!(
            "{}/{}",
            hyperlink(constants::GITHUB_USER, &user_url),
            hyperlink(constants::GITHUB_REPO, &repo_url)
        )
    } else {
        format!("{}/{}", constants::GITHUB_USER, constants::GITHUB_REPO)
    };

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
{cyan}Proxmox ZNC installer v{version}{reset}\n\
\n\
{cyan}{header}{reset}\n",
        pink = "\x1b[38;5;205m",
        cyan = "\x1b[38;5;51m",
        reset = "\x1b[0m",
        version = constants::VERSION,
        header = header,
    );
}

fn hyperlink(label: &str, url: &str) -> String {
    format!("\x1b]8;;{url}\x1b\\{label}\x1b]8;;\x1b\\")
}

fn hyperlinks_supported() -> bool {
    !matches!(env::var("TERM").ok().as_deref(), Some("dumb"))
}
