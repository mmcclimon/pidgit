fn main() {
  let res = pidgit::run_from_env();

  if let Err(err) = res {
    eprintln!("fatal: {}", err);
    std::process::exit(1);
  }
}
