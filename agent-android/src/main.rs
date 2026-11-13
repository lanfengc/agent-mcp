mod adb;
mod adb_discovery;
mod args;
mod bounds;
mod cli;
mod commands;
mod device;
mod error;
mod mcp;
mod output;
mod policy;
mod shell;
mod state;
mod tool_definitions;
mod tools;

use crate::adb::Adb;
use crate::cli::{Cli, Commands};
use crate::commands::{Ctx, apps, doctor, input, screen, snapshot, wait};
use crate::error::Result;
use crate::policy::Policy;
use clap::Parser;
use serde_json::Value;
use std::process::ExitCode;

fn main() -> ExitCode {
    let cli = Cli::parse();
    let home = cli.home.clone().unwrap_or_else(state::default_home);
    let adb = Adb::discover(cli.adb.clone(), cli.serial.clone());
    if cli.verbose {
        eprintln!(
            "agent-android {} home={}",
            env!("CARGO_PKG_VERSION"),
            home.display()
        );
        match adb.exe_display() {
            Some(path) => eprintln!("adb={path}"),
            None => {
                if let Some(error) = adb.discovery_error() {
                    eprintln!("adb={error}");
                }
            }
        }
    }
    let policy = Policy::from_env();
    let ctx = Ctx {
        adb: &adb,
        policy: &policy,
        home: &home,
    };
    match cli.command {
        Commands::Devices => finish("devices", commands::device::list(&ctx)),
        Commands::Info => finish("info", commands::device::info(&ctx)),
        Commands::Doctor => finish("doctor", doctor::doctor(&ctx)),
        Commands::Screenshot(args) => finish(
            "screenshot",
            screen::screenshot(&ctx, &args).map(|output| output.data),
        ),
        Commands::Snapshot(args) => finish("snapshot", snapshot::snapshot(&ctx, &args)),
        Commands::Find(args) => finish("find", snapshot::find(&ctx, &args)),
        Commands::Tap(args) => finish("tap", input::tap(&ctx, &args)),
        Commands::LongPress(args) => finish("long_press", input::long_press(&ctx, &args)),
        Commands::Swipe(args) => finish("swipe", input::swipe(&ctx, &args)),
        Commands::Type(args) => finish("type", input::type_text(&ctx, &args)),
        Commands::Key(args) => finish("key", input::key(&ctx, &args)),
        Commands::Scroll(args) => finish("scroll", input::scroll(&ctx, &args)),
        Commands::Wait(args) => finish("wait", wait::wait(&ctx, &args)),
        Commands::Apps(args) => finish("apps", apps::list(&ctx, &args)),
        Commands::Foreground => finish("foreground", apps::foreground(&ctx)),
        Commands::Launch(args) => finish("launch", apps::launch(&ctx, &args.package)),
        Commands::Terminate(args) => finish("terminate", apps::terminate(&ctx, &args.package)),
        Commands::Install(args) => finish("install", apps::install(&ctx, &args)),
        Commands::Uninstall(args) => finish("uninstall", apps::uninstall(&ctx, &args)),
        Commands::ClearData(args) => finish("clear_data", apps::clear_data(&ctx, &args)),
        Commands::Logcat(args) => finish("logcat", apps::logcat(&ctx, &args)),
        Commands::OpenUrl(args) => finish("open_url", apps::open_url(&ctx, &args)),
        Commands::Connect(args) => finish("connect", apps::connect(&ctx, &args)),
        Commands::Mcp => mcp::serve(&ctx),
    }
}

fn finish(command: &str, result: Result<Value>) -> ExitCode {
    match result {
        Ok(data) => {
            println!("{}", output::success(command, data));
            ExitCode::SUCCESS
        }
        Err(error) => {
            println!("{}", output::failure(command, &error));
            ExitCode::FAILURE
        }
    }
}
