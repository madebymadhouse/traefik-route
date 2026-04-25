use anyhow::{bail, Context, Result};
use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

const DEFAULT_DIR: &str = "/data/coolify/proxy/dynamic";

#[derive(Parser)]
#[command(
    name = "traefik-route",
    about = "Manage Traefik custom domain routes for Coolify-hosted apps",
    version
)]
struct Cli {
    #[arg(long, default_value = DEFAULT_DIR, help = "Traefik dynamic config directory")]
    dir: PathBuf,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    #[command(about = "Add a domain -> port route")]
    Add {
        #[arg(help = "Domain to route (e.g. myapp.com)")]
        domain: String,
        #[arg(help = "Host port your app is bound to (e.g. 3000)")]
        port: u16,
        #[arg(long, help = "Also route www.<domain>")]
        www: bool,
        #[arg(long, help = "Disable HTTPS / Let's Encrypt (HTTP only)")]
        no_https: bool,
    },
    #[command(about = "Remove a route by domain")]
    Remove {
        domain: String,
    },
    #[command(about = "List all routes managed by traefik-route")]
    List,
    #[command(about = "Print the config that would be written, without writing it")]
    Dry {
        domain: String,
        port: u16,
        #[arg(long)]
        www: bool,
        #[arg(long)]
        no_https: bool,
    },
}

#[derive(Debug, Serialize, Deserialize)]
struct TraefikConfig {
    http: Http,
}

#[derive(Debug, Serialize, Deserialize)]
struct Http {
    routers: HashMap<String, Router>,
    services: HashMap<String, Service>,
}

#[derive(Debug, Serialize, Deserialize)]
struct Router {
    rule: String,
    #[serde(rename = "entryPoints")]
    entry_points: Vec<String>,
    service: String,
    priority: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    tls: Option<Tls>,
}

#[derive(Debug, Serialize, Deserialize)]
struct Tls {
    #[serde(rename = "certResolver")]
    cert_resolver: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct Service {
    #[serde(rename = "loadBalancer")]
    load_balancer: LoadBalancer,
}

#[derive(Debug, Serialize, Deserialize)]
struct LoadBalancer {
    servers: Vec<Server>,
    #[serde(rename = "passHostHeader")]
    pass_host_header: bool,
}

#[derive(Debug, Serialize, Deserialize)]
struct Server {
    url: String,
}

fn slug(domain: &str) -> String {
    domain.replace('.', "-").replace('*', "star")
}

fn host_rule(domain: &str, www: bool) -> String {
    if www {
        format!("Host(`{}`) || Host(`www.{}`)", domain, domain)
    } else {
        format!("Host(`{}`)", domain)
    }
}

fn build_config(domain: &str, port: u16, www: bool, https: bool) -> TraefikConfig {
    let s = slug(domain);
    let svc_name = format!("{}-svc", s);
    let rule = host_rule(domain, www);
    let url = format!("http://host.docker.internal:{}", port);

    let mut routers = HashMap::new();
    let mut services = HashMap::new();

    routers.insert(
        format!("{}-http", s),
        Router {
            rule: rule.clone(),
            entry_points: vec!["http".to_string()],
            service: svc_name.clone(),
            priority: 10,
            tls: None,
        },
    );

    if https {
        routers.insert(
            format!("{}-https", s),
            Router {
                rule,
                entry_points: vec!["https".to_string()],
                service: svc_name.clone(),
                priority: 10,
                tls: Some(Tls {
                    cert_resolver: "letsencrypt".to_string(),
                }),
            },
        );
    }

    services.insert(
        svc_name,
        Service {
            load_balancer: LoadBalancer {
                servers: vec![Server { url }],
                pass_host_header: true,
            },
        },
    );

    TraefikConfig {
        http: Http { routers, services },
    }
}

fn config_path(dir: &Path, domain: &str) -> PathBuf {
    dir.join(format!("{}.yaml", slug(domain)))
}

fn cmd_add(dir: &Path, domain: &str, port: u16, www: bool, https: bool) -> Result<()> {
    fs::create_dir_all(dir).context("creating dynamic config directory")?;

    let path = config_path(dir, domain);
    let config = build_config(domain, port, www, https);
    let yaml = serde_yaml::to_string(&config)?;

    fs::write(&path, &yaml).with_context(|| format!("writing {}", path.display()))?;

    println!("wrote {}", path.display());
    println!();
    println!("test:");
    println!("  curl -H \"Host: {}\" http://localhost/", domain);
    Ok(())
}

fn cmd_remove(dir: &Path, domain: &str) -> Result<()> {
    let path = config_path(dir, domain);
    if !path.exists() {
        bail!("no route file found at {}", path.display());
    }
    fs::remove_file(&path).with_context(|| format!("removing {}", path.display()))?;
    println!("removed {}", path.display());
    Ok(())
}

fn cmd_list(dir: &Path) -> Result<()> {
    if !dir.exists() {
        println!("directory {} does not exist", dir.display());
        return Ok(());
    }

    let mut found = false;
    for entry in fs::read_dir(dir).context("reading dynamic config directory")? {
        let entry = entry?;
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if name == "default_redirect_503.yaml" || name == "Caddyfile" {
            continue;
        }
        if name.ends_with(".yaml") || name.ends_with(".yml") {
            let path = entry.path();
            let content = fs::read_to_string(&path)?;
            if let Ok(config) = serde_yaml::from_str::<TraefikConfig>(&content) {
                for (name, router) in &config.http.routers {
                    println!("{:<40} {}", name, router.rule);
                }
                found = true;
            }
        }
    }

    if !found {
        println!("no routes found in {}", dir.display());
    }
    Ok(())
}

fn cmd_dry(domain: &str, port: u16, www: bool, https: bool) -> Result<()> {
    let config = build_config(domain, port, www, https);
    let yaml = serde_yaml::to_string(&config)?;
    println!("# would write to: /data/coolify/proxy/dynamic/{}.yaml", slug(domain));
    println!("{}", yaml);
    Ok(())
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Command::Add { domain, port, www, no_https } => {
            cmd_add(&cli.dir, &domain, port, www, !no_https)
        }
        Command::Remove { domain } => cmd_remove(&cli.dir, &domain),
        Command::List => cmd_list(&cli.dir),
        Command::Dry { domain, port, www, no_https } => {
            cmd_dry(&domain, port, www, !no_https)
        }
    }
}
