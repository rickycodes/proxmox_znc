use crate::cli::Config;
use crate::constants;
use crate::process::CommandRunner;
use std::fs;

#[derive(Debug, Clone)]
pub struct Spec {
    pub ctid: Option<String>,
    pub container_ip: Option<String>,
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
    pub password: String,
    pub irc_server: String,
    pub irc_port: u16,
    pub irc_network: String,
}

impl From<&Config> for Spec {
    fn from(cfg: &Config) -> Self {
        Self {
            ctid: None,
            container_ip: None,
            hostname: cfg
                .hostname
                .clone()
                .unwrap_or_else(|| constants::DEFAULT_ZNC_NAME.into()),
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
                .unwrap_or_else(|| constants::DEFAULT_ZNC_USER.into()),
            nick: cfg
                .nick
                .clone()
                .unwrap_or_else(|| constants::DEFAULT_NICK.into()),
            alt_nick: cfg.alt_nick.clone().unwrap_or_else(|| {
                format!(
                    "{}_",
                    cfg.znc_user
                        .clone()
                        .unwrap_or_else(|| constants::DEFAULT_ZNC_USER.into())
                )
            }),
            realname: cfg
                .realname
                .clone()
                .unwrap_or_else(|| constants::DEFAULT_NICK.into()),
            password: cfg.password.clone().unwrap_or_default(),
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
        let lines = [
            format!("Would create Alpine LXC with:"),
            String::new(),
            format!("  Hostname: {}", self.hostname),
            format!("  Storage: {}", self.storage),
            format!("  Template storage: {}", self.template_storage),
            format!("  Bridge: {}", self.bridge),
            format!("  Memory: {} MB", self.memory),
            format!("  Swap: {} MB", self.swap),
            format!("  Disk: {} GB", self.disk),
            format!("  Cores: {}", self.cores),
            String::new(),
            format!("Would configure ZNC with:"),
            String::new(),
            format!("  ZNC user: {}", self.znc_user),
            format!("  IRC nick: {}", self.nick),
            format!("  Alt nick: {}", self.alt_nick),
            format!("  Real name: {}", self.realname),
            format!("  IRC network: {}", self.irc_network),
            format!("  IRC server: {}:{}", self.irc_server, self.irc_port),
            String::new(),
            format!("No changes made."),
        ];
        render_cyan_box(&lines);
    }

    pub fn validate_host<R: CommandRunner>(&self, runner: &R) -> Result<(), String> {
        for cmd in ["pct", "pveam"] {
            runner.run_status("sh", &["-lc", &format!("command -v {cmd} >/dev/null")])?;
        }
        Ok(())
    }

    pub fn install<R: CommandRunner>(&mut self, runner: &R) -> Result<(), String> {
        let host_arch = runner.run_owned("uname", &[String::from("-m")])?;
        let template_arch = map_arch(&host_arch)?;
        let ctid = runner.run_owned(
            "pvesh",
            &[String::from("get"), String::from("/cluster/nextid")],
        )?;
        let nameservers = detect_nameservers();
        let template_storage = self.template_storage.clone();
        let template_name = download_alpine_template(runner, &template_storage, &template_arch)?;
        let template_ref = format!("{}:vztmpl/{}", template_storage, template_name);

        self.ctid = Some(ctid.clone());

        create_container(
            runner,
            &ctid,
            &self.hostname,
            &self.storage,
            &template_ref,
            &self.bridge,
            self.memory,
            self.swap,
            self.disk,
            self.cores,
            &nameservers,
        )?;

        runner.run_status_owned("pct", &[String::from("start"), ctid.clone()])?;
        bootstrap_container(
            runner,
            &ctid,
            &nameservers,
            &self.znc_user,
            &self.password,
            &self.nick,
            &self.alt_nick,
            &self.realname,
            &self.irc_network,
            &self.irc_server,
            self.irc_port,
        )?;
        self.container_ip = wait_for_container_ip(runner, &ctid);

        Ok(())
    }

    pub fn print_done(&self) -> Result<(), String> {
        let lines = [
            format!(
                "Container ID: {}",
                self.ctid.as_deref().unwrap_or("unavailable")
            ),
            format!("Hostname: {}", self.hostname),
            format!("IRC server inside ZNC: {}:{}", self.irc_server, self.irc_port),
            format!("IRC nick: {}", self.nick),
            format!("ZNC user: {}", self.znc_user),
            format!(
                "IRC client login format: {}/{}:<password>",
                self.znc_user, self.irc_network
            ),
        ];
        render_cyan_box(&lines);
        Ok(())
    }
}

fn render_cyan_box(lines: &[String]) {
    let pink = "\x1b[38;5;205m";
    let cyan = "\x1b[38;5;51m";
    let reset = "\x1b[0m";
    let width = lines.iter().map(|line| line.len()).max().unwrap_or(0);

    println!();
    println!("{pink}╔{}╗{reset}", "═".repeat(width + 2));
    for line in lines {
        let padded = format!("{line:<width$}", width = width);
        println!("{pink}║{reset}{cyan} {padded} {pink}║{reset}");
    }
    println!("{pink}╚{}╝{reset}", "═".repeat(width + 2));
}

fn map_arch(host_arch: &str) -> Result<String, String> {
    match host_arch.trim() {
        "x86_64" => Ok("amd64".into()),
        "aarch64" => Ok("arm64".into()),
        "armv7l" | "armv7" => Ok("armv7".into()),
        "riscv64" => Ok("riscv64".into()),
        other => Err(format!(
            "unsupported host architecture for Alpine template lookup: {other}"
        )),
    }
}

fn detect_nameservers() -> String {
    let mut servers = Vec::new();

    if let Ok(contents) = fs::read_to_string("/etc/resolv.conf") {
        for line in contents.lines() {
            if let Some(rest) = line.strip_prefix("nameserver ") {
                let ns = rest.split_whitespace().next().unwrap_or("");
                if ns.is_empty() || ns.starts_with("127.") || ns == "::1" {
                    continue;
                }
                if !servers.iter().any(|s| s == ns) {
                    servers.push(ns.to_string());
                }
            }
        }
    }

    if servers.is_empty() {
        servers.push(constants::DEFAULT_PING_TARGET.into());
        servers.push("8.8.8.8".into());
    }

    servers.join(" ")
}

fn download_alpine_template<R: CommandRunner>(
    runner: &R,
    template_storage: &str,
    template_arch: &str,
) -> Result<String, String> {
    runner.run_status("pveam", &["update"])?;
    let available = runner.run_owned("pveam", &[String::from("available")])?;

    let mut matches = Vec::new();
    for line in available.lines() {
        let mut cols = line.split_whitespace();
        let _storage = cols.next();
        let name = cols.next().unwrap_or("");
        if name.starts_with("alpine-")
            && name.ends_with(&format!("_{}.tar.xz", template_arch))
        {
            matches.push(name.to_string());
        }
    }

    matches.sort();
    let template_name = matches
        .pop()
        .ok_or_else(|| format!("could not find an Alpine template for architecture {template_arch}"))?;

    let existing = runner
        .run_owned("pveam", &[String::from("list"), template_storage.to_string()])
        .unwrap_or_default();
    let already_present = existing
        .lines()
        .any(|line| line.split_whitespace().next() == Some(template_name.as_str()));

    if !already_present {
        runner.run_status("pveam", &["download", template_storage, &template_name])?;
    }

    Ok(template_name)
}

fn create_container<R: CommandRunner>(
    runner: &R,
    ctid: &str,
    hostname: &str,
    storage: &str,
    template_ref: &str,
    bridge: &str,
    memory: u32,
    swap: u32,
    disk: u32,
    cores: u32,
    nameservers: &str,
) -> Result<(), String> {
    let rootfs = format!("{storage}:{disk}");
    let net0 = format!("name=eth0,bridge={bridge},ip=dhcp");

    let args = vec![
        String::from("create"),
        ctid.to_string(),
        template_ref.to_string(),
        String::from("--hostname"),
        hostname.to_string(),
        String::from("--ostype"),
        String::from("alpine"),
        String::from("--unprivileged"),
        String::from("1"),
        String::from("--cores"),
        cores.to_string(),
        String::from("--memory"),
        memory.to_string(),
        String::from("--swap"),
        swap.to_string(),
        String::from("--rootfs"),
        rootfs,
        String::from("--net0"),
        net0,
        String::from("--nameserver"),
        nameservers.to_string(),
        String::from("--onboot"),
        String::from("1"),
    ];

    runner.run_status_owned("pct", &args)
}

fn bootstrap_container<R: CommandRunner>(
    runner: &R,
    ctid: &str,
    nameservers: &str,
    znc_user: &str,
    znc_password: &str,
    irc_nick: &str,
    irc_alt_nick: &str,
    irc_realname: &str,
    irc_network: &str,
    irc_server: &str,
    irc_port: u16,
) -> Result<(), String> {
    push_resolv_conf(runner, ctid, nameservers)?;
    wait_for_network(runner, ctid)?;
    install_packages(runner, ctid)?;
    ensure_znc_user(runner, ctid)?;
    ensure_znc_dirs(runner, ctid)?;
    run_makeconf(
        runner,
        ctid,
        znc_user,
        znc_password,
        irc_nick,
        irc_alt_nick,
        irc_realname,
        irc_network,
        irc_server,
        irc_port,
    )?;
    chown_znc_tree(runner, ctid)?;
    enable_service(runner, ctid)?;

    Ok(())
}

fn push_resolv_conf<R: CommandRunner>(
    runner: &R,
    ctid: &str,
    nameservers: &str,
) -> Result<(), String> {
    let mut path = std::env::temp_dir();
    path.push(format!("proxmox-znc-resolv-{ctid}.conf"));

    let content = nameservers
        .split_whitespace()
        .map(|ns| format!("nameserver {ns}\n"))
        .collect::<String>();
    fs::write(&path, content).map_err(|e| e.to_string())?;

    let path_str = path.to_string_lossy().to_string();
    let args = vec![
        String::from("push"),
        ctid.to_string(),
        path_str.clone(),
        String::from("/etc/resolv.conf"),
    ];
    let result = runner.run_status_owned("pct", &args);
    let _ = fs::remove_file(&path);
    result
}

fn wait_for_network<R: CommandRunner>(runner: &R, ctid: &str) -> Result<(), String> {
    for _ in 0..12 {
        let args = vec![
            String::from("exec"),
            ctid.to_string(),
            String::from("--"),
            String::from("ping"),
            String::from("-c"),
            String::from("1"),
            String::from("-W"),
            String::from("1"),
            String::from(constants::DEFAULT_PING_TARGET),
        ];
        if runner.run_status_owned("pct", &args).is_ok() {
            return Ok(());
        }
        std::thread::sleep(std::time::Duration::from_secs(2));
    }
    Ok(())
}

fn install_packages<R: CommandRunner>(runner: &R, ctid: &str) -> Result<(), String> {
    for _ in 0..5 {
        let args = vec![
            String::from("exec"),
            ctid.to_string(),
            String::from("--"),
            String::from("apk"),
            String::from("add"),
            String::from("--no-cache"),
            String::from("ca-certificates"),
            String::from(constants::DEFAULT_ZNC_NAME),
            String::from(constants::ZNC_OPENRC_PACKAGE),
        ];
        if runner.run_status_owned("pct", &args).is_ok() {
            return Ok(());
        }
        std::thread::sleep(std::time::Duration::from_secs(4));
    }
    let args = vec![
        String::from("exec"),
        ctid.to_string(),
        String::from("--"),
        String::from("apk"),
        String::from("add"),
        String::from("--no-cache"),
        String::from("ca-certificates"),
        String::from(constants::DEFAULT_ZNC_NAME),
        String::from(constants::ZNC_OPENRC_PACKAGE),
    ];
    runner.run_status_owned("pct", &args)
}

fn ensure_znc_user<R: CommandRunner>(runner: &R, ctid: &str) -> Result<(), String> {
    let check_args = vec![
        String::from("exec"),
        ctid.to_string(),
        String::from("--"),
        String::from("id"),
        String::from(constants::DEFAULT_ZNC_USER),
    ];
    if runner.run_status_owned("pct", &check_args).is_err() {
        let add_args = vec![
            String::from("exec"),
            ctid.to_string(),
            String::from("--"),
            String::from("adduser"),
            String::from("-D"),
            String::from("-h"),
            String::from("/var/lib/znc"),
            String::from("-s"),
            String::from("/sbin/nologin"),
            String::from(constants::DEFAULT_ZNC_USER),
        ];
        runner.run_status_owned("pct", &add_args)?;
    }
    Ok(())
}

fn ensure_znc_dirs<R: CommandRunner>(runner: &R, ctid: &str) -> Result<(), String> {
    let dirs = [
        "/var/lib/znc",
        "/var/lib/znc/configs",
    ];
    for dir in dirs {
        let args = vec![
            String::from("exec"),
            ctid.to_string(),
            String::from("--"),
            String::from("install"),
            String::from("-d"),
            String::from("-o"),
            String::from(constants::DEFAULT_ZNC_USER),
            String::from("-g"),
            String::from(constants::DEFAULT_ZNC_USER),
            String::from(dir),
        ];
        runner.run_status_owned("pct", &args)?;
    }
    Ok(())
}

fn makeconf_answers(
    znc_user: &str,
    znc_password: &str,
    irc_nick: &str,
    irc_alt_nick: &str,
    irc_realname: &str,
    irc_network: &str,
    irc_server: &str,
    irc_port: u16,
) -> String {
    [
        constants::DEFAULT_ZNC_LISTENER_PORT.to_string(),
        String::from("yes"),
        String::from("yes"),
        String::new(),
        znc_user.to_string(),
        znc_password.to_string(),
        znc_password.to_string(),
        irc_nick.to_string(),
        irc_alt_nick.to_string(),
        znc_user.to_string(),
        irc_realname.to_string(),
        String::new(),
        String::from("yes"),
        irc_network.to_string(),
        irc_server.to_string(),
        String::from("yes"),
        irc_port.to_string(),
        String::new(),
        String::new(),
        String::from("no"),
    ]
    .join("\n")
        + "\n"
}

fn run_makeconf<R: CommandRunner>(
    runner: &R,
    ctid: &str,
    znc_user: &str,
    znc_password: &str,
    irc_nick: &str,
    irc_alt_nick: &str,
    irc_realname: &str,
    irc_network: &str,
    irc_server: &str,
    irc_port: u16,
) -> Result<(), String> {
    let answers = makeconf_answers(
        znc_user,
        znc_password,
        irc_nick,
        irc_alt_nick,
        irc_realname,
        irc_network,
        irc_server,
        irc_port,
    );
    let args = vec![
        String::from("exec"),
        ctid.to_string(),
        String::from("--"),
        String::from("su"),
        String::from("-s"),
        String::from("/bin/sh"),
        String::from(constants::DEFAULT_ZNC_USER),
        String::from("-c"),
        String::from("HOME=/var/lib/znc znc --datadir=/var/lib/znc --makeconf"),
    ];
    runner.run_status_owned_with_input("pct", &args, &answers)
}

fn chown_znc_tree<R: CommandRunner>(runner: &R, ctid: &str) -> Result<(), String> {
    let args = vec![
        String::from("exec"),
        ctid.to_string(),
        String::from("--"),
        String::from("chown"),
        String::from("-R"),
        String::from(constants::DEFAULT_ZNC_USER),
        String::from("/var/lib/znc"),
    ];
    runner.run_status_owned("pct", &args)
}

fn enable_service<R: CommandRunner>(runner: &R, ctid: &str) -> Result<(), String> {
    let add_args = vec![
        String::from("exec"),
        ctid.to_string(),
        String::from("--"),
        String::from("rc-update"),
        String::from("add"),
        String::from(constants::DEFAULT_ZNC_USER),
        String::from("default"),
    ];
    runner.run_status_owned("pct", &add_args)?;

    let start_args = vec![
        String::from("exec"),
        ctid.to_string(),
        String::from("--"),
        String::from("rc-service"),
        String::from(constants::DEFAULT_ZNC_USER),
        String::from("start"),
    ];
    runner.run_status_owned("pct", &start_args)
}

fn wait_for_container_ip<R: CommandRunner>(runner: &R, ctid: &str) -> Option<String> {
    for _ in 0..15 {
        if let Ok(output) = runner.run("pct", &["exec", ctid, "--", "hostname", "-I"]) {
            if let Some(ip) = output.split_whitespace().next() {
                if !ip.is_empty() {
                    return Some(ip.to_string());
                }
            }
        }
        std::thread::sleep(std::time::Duration::from_secs(2));
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn makeconf_answers_includes_password_twice() {
        let answers = makeconf_answers(
            "znc",
            "secret",
            "nick",
            "nick_",
            "real",
            "libera",
            "irc.libera.chat",
            6697,
        );

        let lines: Vec<&str> = answers.lines().collect();
        assert_eq!(lines[0], constants::DEFAULT_ZNC_LISTENER_PORT.to_string());
        assert_eq!(lines[4], "znc");
        assert_eq!(lines[5], "secret");
        assert_eq!(lines[6], "secret");
        assert_eq!(lines[7], "nick");
        assert_eq!(lines[8], "nick_");
    }
}
