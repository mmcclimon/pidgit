mod cmd;

use clap::{crate_version, App, AppSettings};

fn main() {
  let matches = App::new("pidgit")
    .version(crate_version!())
    .setting(AppSettings::SubcommandRequiredElseHelp)
    .subcommands(cmd::command_apps())
    .get_matches();

  cmd::dispatch(&matches);
}
