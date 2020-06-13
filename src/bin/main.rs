fn main() {
  let matches = pidgit::app().get_matches();
  let mut stdout = std::io::stdout();
  let res = pidgit::cmd::dispatch(&matches, &mut stdout);

  if let Err(err) = res {
    eprintln!("fatal: {}", err);
    std::process::exit(1);
  }
}
