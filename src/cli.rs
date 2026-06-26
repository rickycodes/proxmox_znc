use crate::constants;
use crate::prompt;
use clap::Parser;

#[derive(Debug, Clone, Parser)]
#[command(
    name = "proxmox-znc",
    about = "Interactive Proxmox LXC installer for a basic ZNC bounce"
)]
pub struct Config {
    #[arg(long)]
    pub dry_run: bool,
    #[arg(long, env = "HOSTNAME")]
    pub hostname: Option<String>,
    #[arg(long, env = "STORAGE")]
    pub storage: Option<String>,
    #[arg(long = "template-storage", env = "TEMPLATE_STORAGE")]
    pub template_storage: Option<String>,
    #[arg(long, env = "BRIDGE")]
    pub bridge: Option<String>,
    #[arg(long, env = "MEMORY")]
    pub memory: Option<u32>,
    #[arg(long, env = "SWAP")]
    pub swap: Option<u32>,
    #[arg(long, env = "DISK")]
    pub disk: Option<u32>,
    #[arg(long, env = "CORES")]
    pub cores: Option<u32>,
    #[arg(long = "znc-user", env = "ZNC_USER")]
    pub znc_user: Option<String>,
    #[arg(long, env = "NICK")]
    pub nick: Option<String>,
    #[arg(long = "alt-nick", env = "ALT_NICK")]
    pub alt_nick: Option<String>,
    #[arg(long, env = "REALNAME")]
    pub realname: Option<String>,
    #[arg(long, env = "PASSWORD")]
    pub password: Option<String>,
    #[arg(long = "irc-server", env = "IRC_SERVER")]
    pub irc_server: Option<String>,
    #[arg(long = "irc-port", env = "IRC_PORT")]
    pub irc_port: Option<u16>,
    #[arg(long = "irc-network", env = "IRC_NETWORK")]
    pub irc_network: Option<String>,
}

impl Config {
    pub fn from_env_and_args() -> Result<Self, String> {
        Self::try_parse().map_err(|e| e.to_string())
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
            prompt::choose(
                "Root disk storage",
                default_idx,
                &storages,
                &mut self.storage,
            )?;
        }
        let template_storages =
            crate::storage::detect_template_storages(runner).unwrap_or_default();
        if template_storages.is_empty() {
            prompt::text(
                "Template storage",
                constants::DEFAULT_TEMPLATE_STORAGE,
                &mut self.template_storage,
            )?;
        } else if template_storages.len() == 1 {
            self.template_storage = Some(template_storages[0].clone());
        } else {
            let default_idx = template_storages
                .iter()
                .position(|s| s == constants::DEFAULT_TEMPLATE_STORAGE)
                .unwrap_or(0);
            prompt::choose(
                "Template storage",
                default_idx,
                &template_storages,
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
        let default_nick = default_nick_from_hostname(self.hostname.as_deref());
        prompt::text("IRC nick", &default_nick, &mut self.nick)?;
        prompt::text(
            "ZNC admin username",
            self.nick.as_deref().unwrap_or(constants::DEFAULT_ZNC_USER),
            &mut self.znc_user,
        )?;
        let alt_default = format!(
            "{}_",
            self.znc_user
                .as_deref()
                .unwrap_or(constants::DEFAULT_ZNC_USER)
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

fn default_nick_from_hostname(hostname: Option<&str>) -> String {
    hostname.unwrap_or(constants::DEFAULT_NICK).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_nick_prefers_hostname() {
        assert_eq!(default_nick_from_hostname(Some("alpha")), "alpha");
        assert_eq!(default_nick_from_hostname(None), constants::DEFAULT_NICK);
    }
}
