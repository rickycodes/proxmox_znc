use crate::cli::Config;
use crate::constants;
use crate::process::CommandRunner;

#[derive(Debug, Clone)]
pub struct Spec {
    pub hostname: String,
    pub storage: String,
    pub template_storage: String,
    pub bridge: String,
    pub memory: u32,
    pub swap: u32,
    pub disk: u32,
    pub cores: u32,
    pub znc_user: String,
    pub nick: String,
    pub alt_nick: String,
    pub realname: String,
    pub irc_server: String,
    pub irc_port: u16,
    pub irc_network: String,
}

impl From<&Config> for Spec {
    fn from(cfg: &Config) -> Self {
        Self {
            hostname: cfg
                .hostname
                .clone()
                .unwrap_or_else(|| constants::DEFAULT_CONTAINER_HOSTNAME.into()),
            storage: cfg
                .storage
                .clone()
                .unwrap_or_else(|| constants::DEFAULT_STORAGE.into()),
            template_storage: cfg
                .template_storage
                .clone()
                .unwrap_or_else(|| constants::DEFAULT_TEMPLATE_STORAGE.into()),
            bridge: cfg
                .bridge
                .clone()
                .unwrap_or_else(|| constants::DEFAULT_BRIDGE.into()),
            memory: cfg.memory.unwrap_or(constants::DEFAULT_MEMORY_MB),
            swap: cfg.swap.unwrap_or(constants::DEFAULT_SWAP_MB),
            disk: cfg.disk.unwrap_or(constants::DEFAULT_DISK_GB),
            cores: cfg.cores.unwrap_or(constants::DEFAULT_CORES),
            znc_user: cfg
                .znc_user
                .clone()
                .unwrap_or_else(|| constants::DEFAULT_NICK.into()),
            nick: cfg
                .nick
                .clone()
                .unwrap_or_else(|| constants::DEFAULT_NICK.into()),
            alt_nick: cfg.alt_nick.clone().unwrap_or_else(|| {
                format!(
                    "{}_",
                    cfg.znc_user
                        .clone()
                        .unwrap_or_else(|| constants::DEFAULT_NICK.into())
                )
            }),
            realname: cfg
                .realname
                .clone()
                .unwrap_or_else(|| constants::DEFAULT_NICK.into()),
            irc_server: cfg
                .irc_server
                .clone()
                .unwrap_or_else(|| constants::DEFAULT_IRC_SERVER.into()),
            irc_port: cfg.irc_port.unwrap_or(constants::DEFAULT_IRC_PORT),
            irc_network: cfg
                .irc_network
                .clone()
                .unwrap_or_else(|| constants::DEFAULT_IRC_NETWORK.into()),
        }
    }
}

impl Spec {
    pub fn print(&self) {
        println!("Would create Alpine LXC with:");
        println!("  Hostname: {}", self.hostname);
        println!("  Storage: {}", self.storage);
        println!("  Template storage: {}", self.template_storage);
        println!("  Bridge: {}", self.bridge);
        println!("  Memory: {} MB", self.memory);
        println!("  Swap: {} MB", self.swap);
        println!("  Disk: {} GB", self.disk);
        println!("  Cores: {}", self.cores);
        println!("Would configure ZNC with:");
        println!("  ZNC user: {}", self.znc_user);
        println!("  IRC nick: {}", self.nick);
        println!("  Alt nick: {}", self.alt_nick);
        println!("  Real name: {}", self.realname);
        println!("  IRC network: {}", self.irc_network);
        println!("  IRC server: {}:{}", self.irc_server, self.irc_port);
        println!("No changes made.");
    }

    pub fn validate_host<R: CommandRunner>(&self, runner: &R) -> Result<(), String> {
        for cmd in ["pct", "pveam"] {
            runner.run_status("sh", &["-lc", &format!("command -v {cmd} >/dev/null")])?;
        }
        Ok(())
    }

    pub fn install<R: CommandRunner>(&self, runner: &R) -> Result<(), String> {
        let _ = runner.run("uname", &["-m"])?;
        let _ = runner.run("pvesh", &["get", "/cluster/nextid"])?;
        Ok(())
    }

    pub fn print_done<R: CommandRunner>(&self, _runner: &R) -> Result<(), String> {
        println!();
        println!("Container ID: auto");
        println!("Hostname: {}", self.hostname);
        println!(
            "IRC server inside ZNC: {}:{}",
            self.irc_server, self.irc_port
        );
        println!("IRC nick: {}", self.nick);
        println!("ZNC user: {}", self.znc_user);
        println!(
            "IRC client login format: {}/{}:<password>",
            self.znc_user, self.irc_network
        );
        Ok(())
    }
}
