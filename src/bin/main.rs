fn main() {
  let cmd = pidgit::cmd::CommandSet::new();
  let matches = cmd.app().get_matches();
  let mut stdout = std::io::stdout();
  let res = cmd.dispatch(&matches, &mut stdout);

  if let Err(err) = res {
    eprintln!("fatal: {}", err);
    std::process::exit(1);
  }
}
