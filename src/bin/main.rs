fn main() {
  let matches = pidgit::app().get_matches();
  let res = pidgit::cmd::dispatch(&matches);

  if let Err(err) = res {
    eprintln!("fatal: {}", err);
    std::process::exit(1);
  }
}
