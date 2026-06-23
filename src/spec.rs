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
        println!();
        println!(
            "Container ID: {}",
            self.ctid.as_deref().unwrap_or("unavailable")
        );
        println!("Hostname: {}", self.hostname);
        if let Some(ip) = &self.container_ip {
            println!("Container IP: {}", ip);
            println!("ZNC listener: {}:{}", ip, self.irc_port);
        } else {
            println!("Container IP: unavailable yet");
        }
        println!("IRC server inside ZNC: {}:{}", self.irc_server, self.irc_port);
        println!("IRC nick: {}", self.nick);
        println!("ZNC user: {}", self.znc_user);
        println!(
            "IRC client login format: {}/{}:<password>",
            self.znc_user, self.irc_network
        );
        Ok(())
    }
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
        servers.push("1.1.1.1".into());
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
    let script = r#"
set -eu

if [ -n "${NAMESERVERS:-}" ]; then
  : > /etc/resolv.conf
  for ns in $NAMESERVERS; do
    printf 'nameserver %s\n' "$ns" >> /etc/resolv.conf
  done
fi

wait_for_network() {
  i=0
  while [ "$i" -lt 12 ]; do
    if ping -c 1 -W 1 1.1.1.1 >/dev/null 2>&1; then
      return 0
    fi
    i=$((i + 1))
    sleep 2
  done
  return 1
}

wait_for_network || true

i=0
while :; do
  if apk add --no-cache ca-certificates znc znc-openrc >/dev/null 2>&1; then
    break
  fi
  i=$((i + 1))
  if [ "$i" -ge 5 ]; then
    apk add --no-cache ca-certificates znc znc-openrc
    exit 1
  fi
  sleep 4
done

if ! id znc >/dev/null 2>&1; then
  adduser -D -h /var/lib/znc -s /sbin/nologin znc
fi

install -d -o znc -g znc /var/lib/znc
install -d -o znc -g znc /var/lib/znc/configs

answers="$(mktemp)"
trap 'rm -f "$answers"' EXIT

{
  printf '%s\n' \
    '6697' \
    'yes' \
    'yes' \
    '' \
    "$ZNC_USER" \
    "$ZNC_PASSWORD" \
    "$ZNC_PASSWORD" \
    "$IRC_NICK" \
    "$IRC_ALT_NICK" \
    "$ZNC_USER" \
    "$IRC_REALNAME" \
    '' \
    'yes' \
    "$IRC_NETWORK" \
    "$IRC_SERVER" \
    'yes' \
    "$IRC_PORT" \
    '' \
    '' \
    'no'
} >"$answers"

chown znc:znc "$answers"

su -s /bin/sh znc -c "HOME=/var/lib/znc znc --datadir=/var/lib/znc --makeconf" <"$answers" >/tmp/znc-makeconf.log 2>&1 || {
  cat /tmp/znc-makeconf.log >&2
  exit 1
}

chown -R znc:znc /var/lib/znc
rc-update add znc default >/dev/null
rc-service znc start >/dev/null
"#;

    let args = vec![
        String::from("exec"),
        ctid.to_string(),
        String::from("--"),
        String::from("env"),
        format!("NAMESERVERS={nameservers}"),
        format!("ZNC_USER={znc_user}"),
        format!("ZNC_PASSWORD={znc_password}"),
        format!("IRC_NICK={irc_nick}"),
        format!("IRC_ALT_NICK={irc_alt_nick}"),
        format!("IRC_REALNAME={irc_realname}"),
        format!("IRC_NETWORK={irc_network}"),
        format!("IRC_SERVER={irc_server}"),
        format!("IRC_PORT={irc_port}"),
        String::from("/bin/sh"),
        String::from("-c"),
        script.to_string(),
    ];

    runner.run_status_owned("pct", &args)
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
