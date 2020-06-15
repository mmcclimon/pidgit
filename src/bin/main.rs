fn main() {
  let app = pidgit::new();
  let repo = pidgit::util::find_repo();
  let res = app.dispatch(&app.get_matches(), repo.as_ref(), std::io::stdout());

  if let Err(err) = res {
    eprintln!("fatal: {}", err);
    std::process::exit(1);
  }
}
