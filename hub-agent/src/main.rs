//! CLI: skillbasin-agent init|apply [--config <path>]
//! Argument surface is deliberately tiny; the config file carries the rest.

use std::path::PathBuf;
use std::process::ExitCode;

fn default_config_path() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".skillbasin")
        .join("agent.json")
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut config_path = default_config_path();
    let mut mode: Option<&str> = None;
    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "init" | "apply" => mode = Some(Box::leak(arg.clone().into_boxed_str())),
            "--config" => match iter.next() {
                Some(p) => config_path = PathBuf::from(p),
                None => {
                    eprintln!("--config needs a path");
                    return ExitCode::from(2);
                }
            },
            other => {
                eprintln!("unknown argument: {other}");
                return ExitCode::from(2);
            }
        }
    }

    let Some(mode) = mode else {
        eprintln!("usage: skillbasin-agent <init|apply> [--config <path>]");
        return ExitCode::from(2);
    };

    let config = match hub_agent::load_config(&config_path) {
        Ok(c) => c,
        Err(err) => {
            eprintln!("config error ({}): {err:#}", config_path.display());
            return ExitCode::FAILURE;
        }
    };

    let result = match mode {
        "init" => hub_agent::run_init(&config).map(|()| true),
        _ => hub_agent::run_apply(&config).map(|report| {
            for a in &report.actions {
                let mark = if a.ok { "ok " } else { "ERR" };
                match &a.error {
                    Some(e) => eprintln!("[{mark}] {} {} -> {}: {e}", a.action, a.skill, a.tool),
                    None => eprintln!("[{mark}] {} {} -> {}", a.action, a.skill, a.tool),
                }
            }
            report.ok
        }),
    };

    match result {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::FAILURE, // partial failure is visible to cron/systemd too
        Err(err) => {
            eprintln!("{mode} failed: {err:#}");
            ExitCode::FAILURE
        }
    }
}
