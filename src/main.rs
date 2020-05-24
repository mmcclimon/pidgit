use clap::{crate_version, App, AppSettings};

fn main() {
  let matches = App::new("pidgit")
    .version(crate_version!())
    .settings(&[
      AppSettings::SubcommandRequiredElseHelp,
      AppSettings::VersionlessSubcommands,
    ])
    .subcommands(pidgit::cmd::command_apps())
    .get_matches();

  let res = pidgit::cmd::dispatch(&matches);

  if let Err(err) = res {
    eprintln!("fatal: {}", err);
    std::process::exit(1);
  }
}
