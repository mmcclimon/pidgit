fn main() {
  let app = pidgit::new();
  let res = app.dispatch(&app.get_matches(), std::io::stdout());

  if let Err(err) = res {
    eprintln!("fatal: {}", err);
    std::process::exit(1);
  }
}
