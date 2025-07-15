use anyhow::{Result, bail};

#[derive(Debug, Clone, Default)]
pub struct Config {
    pub paths: Vec<String>,
    pub derivation: bool,
    pub store: Option<String>,
    pub help: bool,
    pub version: bool,
    pub nix_options: Vec<(String, String)>,
    pub file: Option<String>,
}

pub fn parse_args() -> Result<Config> {
    let args: Vec<String> = std::env::args().collect();
    let mut config = Config::default();

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "-h" | "--help" => {
                config.help = true;
                return Ok(config);
            }
            "-v" | "--version" => {
                config.version = true;
                return Ok(config);
            }
            "-d" | "--derivation" => {
                config.derivation = true;
            }
            "--store" => {
                i += 1;
                if i >= args.len() {
                    bail!("--store requires an argument");
                }
                config.store = Some(args[i].clone());
            }
            arg if arg.starts_with("--store=") => {
                config.store = Some(arg.strip_prefix("--store=").unwrap().to_string());
            }
            "--option" => {
                i += 1;
                if i + 1 >= args.len() {
                    bail!("--option requires two arguments: name and value");
                }
                let name = args[i].clone();
                i += 1;
                let value = args[i].clone();
                config.nix_options.push((name, value));
            }
            "-f" | "--file" => {
                i += 1;
                if i >= args.len() {
                    bail!("--file requires an argument");
                }
                config.file = Some(args[i].clone());
            }
            arg if arg.starts_with("--file=") => {
                config.file = Some(arg.strip_prefix("--file=").unwrap().to_string());
            }
            arg if arg.starts_with('-') => {
                bail!("Unknown option: {}", arg);
            }
            _ => {
                config.paths.push(args[i].clone());
            }
        }
        i += 1;
    }

    Ok(config)
}

pub fn print_help() {
    println!(
        r#"nix-tree - Interactively browse dependency graphs of Nix derivations

USAGE:
    nix-tree [OPTIONS] [PATHS]...

OPTIONS:
    -h, --help              Display help message
    -v, --version           Display version
    -d, --derivation        Operate on derivation store paths
    --store <STORE>         The URL of the Nix store, e.g. "daemon" or "https://cache.nixos.org"
                            See "nix help-stores" for supported store types and settings
    --option <NAME> <VALUE> Pass option to nix commands
    -f, --file <FILE>       Interpret installables as attribute paths relative to the Nix expression in file

ARGUMENTS:
    [PATHS]...          Paths to explore (defaults to current system profile)

KEYBINDINGS:
    q/Esc               Quit
    j/Down              Move down
    k/Up                Move up
    h/Left              Move to previous pane  
    l/Right             Move to next pane
    /                   Search
    s                   Change sort order
    ?                   Show help
"#
    );
}

pub fn print_version() {
    println!("nix-tree {}", env!("CARGO_PKG_VERSION"));
}
