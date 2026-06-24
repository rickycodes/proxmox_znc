use crate::constants;
use crate::prompt;
use std::env;

#[derive(Debug, Clone)]
pub struct Config {
    pub dry_run: bool,
    pub hostname: Option<String>,
    pub storage: Option<String>,
    pub template_storage: Option<String>,
    pub bridge: Option<String>,
    pub memory: Option<u32>,
    pub swap: Option<u32>,
    pub disk: Option<u32>,
    pub cores: Option<u32>,
    pub znc_user: Option<String>,
    pub nick: Option<String>,
    pub alt_nick: Option<String>,
    pub realname: Option<String>,
    pub password: Option<String>,
    pub irc_server: Option<String>,
    pub irc_port: Option<u16>,
    pub irc_network: Option<String>,
}

impl Config {
    pub fn from_env_and_args() -> Result<Self, String> {
        let mut cfg = Self {
            dry_run: false,
            hostname: env_opt("HOSTNAME"),
            storage: env_opt("STORAGE"),
            template_storage: env_opt("TEMPLATE_STORAGE"),
            bridge: env_opt("BRIDGE"),
            memory: env_parse("MEMORY"),
            swap: env_parse("SWAP"),
            disk: env_parse("DISK"),
            cores: env_parse("CORES"),
            znc_user: env_opt("ZNC_USER"),
            nick: env_opt("NICK"),
            alt_nick: env_opt("ALT_NICK"),
            realname: env_opt("REALNAME"),
            password: env_opt("PASSWORD"),
            irc_server: env_opt("IRC_SERVER"),
            irc_port: env_parse("IRC_PORT"),
            irc_network: env_opt("IRC_NETWORK"),
        };

        let mut args = env::args().skip(1);
        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--dry-run" => cfg.dry_run = true,
                "--hostname" => cfg.hostname = Some(next_value(&mut args, "--hostname")?),
                "--storage" => cfg.storage = Some(next_value(&mut args, "--storage")?),
                "--template-storage" => {
                    cfg.template_storage = Some(next_value(&mut args, "--template-storage")?)
                }
                "--bridge" => cfg.bridge = Some(next_value(&mut args, "--bridge")?),
                "--memory" => cfg.memory = Some(parse_u32(&next_value(&mut args, "--memory")?)?),
                "--swap" => cfg.swap = Some(parse_u32(&next_value(&mut args, "--swap")?)?),
                "--disk" => cfg.disk = Some(parse_u32(&next_value(&mut args, "--disk")?)?),
                "--cores" => cfg.cores = Some(parse_u32(&next_value(&mut args, "--cores")?)?),
                "--znc-user" => cfg.znc_user = Some(next_value(&mut args, "--znc-user")?),
                "--nick" => cfg.nick = Some(next_value(&mut args, "--nick")?),
                "--alt-nick" => cfg.alt_nick = Some(next_value(&mut args, "--alt-nick")?),
                "--realname" => cfg.realname = Some(next_value(&mut args, "--realname")?),
                "--password" => cfg.password = Some(next_value(&mut args, "--password")?),
                "--irc-server" => cfg.irc_server = Some(next_value(&mut args, "--irc-server")?),
                "--irc-port" => {
                    cfg.irc_port = Some(parse_u16(&next_value(&mut args, "--irc-port")?)?)
                }
                "--irc-network" => cfg.irc_network = Some(next_value(&mut args, "--irc-network")?),
                "--help" | "-h" => {
                    print_usage();
                    std::process::exit(0);
                }
                other => return Err(format!("unknown argument: {other}")),
            }
        }

        Ok(cfg)
    }

    pub fn prompt_missing<R: crate::process::CommandRunner>(
        &mut self,
        runner: &R,
    ) -> Result<(), String> {
        prompt::text(
            "Container hostname",
            constants::DEFAULT_ZNC_NAME,
            &mut self.hostname,
        )?;
        let storages = crate::storage::detect_storages(runner).unwrap_or_default();
        if storages.is_empty() {
            prompt::text(
                "Root disk storage",
                constants::DEFAULT_STORAGE,
                &mut self.storage,
            )?;
            prompt::text(
                "Template storage",
                constants::DEFAULT_TEMPLATE_STORAGE,
                &mut self.template_storage,
            )?;
        } else {
            let default_idx = storages
                .iter()
                .position(|s| s == constants::DEFAULT_STORAGE)
                .unwrap_or(0);
            prompt::choose("Root disk storage", default_idx, &storages, &mut self.storage)?;
            prompt::choose(
                "Template storage",
                default_idx,
                &storages,
                &mut self.template_storage,
            )?;
        }
        prompt::text(
            "Proxmox bridge",
            constants::DEFAULT_BRIDGE,
            &mut self.bridge,
        )?;
        prompt::number(
            "Container RAM (MB)",
            constants::DEFAULT_MEMORY_MB,
            &mut self.memory,
        )?;
        prompt::number(
            "Container swap (MB)",
            constants::DEFAULT_SWAP_MB,
            &mut self.swap,
        )?;
        prompt::number(
            "Container root disk (GB)",
            constants::DEFAULT_DISK_GB,
            &mut self.disk,
        )?;
        prompt::number(
            "Container CPU cores",
            constants::DEFAULT_CORES,
            &mut self.cores,
        )?;
        prompt::text("IRC nick", constants::DEFAULT_NICK, &mut self.nick)?;
        prompt::text(
            "ZNC admin username",
            self.nick.as_deref().unwrap_or(constants::DEFAULT_ZNC_USER),
            &mut self.znc_user,
        )?;
        let alt_default = format!(
            "{}_",
            self.znc_user.as_deref().unwrap_or(constants::DEFAULT_ZNC_USER)
        );
        prompt::text("IRC alternate nick", &alt_default, &mut self.alt_nick)?;
        prompt::text(
            "IRC real name",
            self.nick.as_deref().unwrap_or(constants::DEFAULT_NICK),
            &mut self.realname,
        )?;
        prompt::text(
            "IRC server",
            constants::DEFAULT_IRC_SERVER,
            &mut self.irc_server,
        )?;
        prompt::number(
            "IRC server port",
            constants::DEFAULT_IRC_PORT,
            &mut self.irc_port,
        )?;
        prompt::text(
            "IRC network name",
            constants::DEFAULT_IRC_NETWORK,
            &mut self.irc_network,
        )?;
        prompt::secret("ZNC password", &mut self.password)?;
        Ok(())
    }
}

fn print_usage() {
    println!(
        "Usage: proxmox-znc [options]\n\n\
         Interactive Proxmox LXC installer for a basic ZNC bounce.\n\n\
         Options:\n\
           --dry-run\n\
           --hostname NAME\n\
           --storage NAME\n\
           --template-storage NAME\n\
           --bridge NAME\n\
           --memory MB\n\
           --swap MB\n\
           --disk GB\n\
           --cores N\n\
           --znc-user NAME\n\
           --nick NAME\n\
           --alt-nick NAME\n\
           --realname NAME\n\
           --password PASS\n\
           --irc-server HOST\n\
           --irc-port PORT\n\
           --irc-network NAME"
    );
}

fn env_opt(key: &str) -> Option<String> {
    env::var(key).ok().filter(|s| !s.is_empty())
}

fn env_parse<T: std::str::FromStr>(key: &str) -> Option<T> {
    env::var(key).ok().and_then(|s| s.parse().ok())
}

fn next_value(args: &mut impl Iterator<Item = String>, flag: &str) -> Result<String, String> {
    args.next()
        .ok_or_else(|| format!("{flag} requires a value"))
}

fn parse_u32(s: &str) -> Result<u32, String> {
    s.parse().map_err(|_| format!("invalid number: {s}"))
}

fn parse_u16(s: &str) -> Result<u16, String> {
    s.parse().map_err(|_| format!("invalid number: {s}"))
}
