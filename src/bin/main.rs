fn main() {
  pretty_env_logger::init();

  let res = pidgit::run_from_env();

  if let Err(err) = res {
    eprintln!("fatal: {}", err);
    std::process::exit(1);
  }
}
