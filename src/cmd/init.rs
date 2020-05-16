use clap::{App, ArgMatches};

pub fn app<'a, 'b>() -> App<'a, 'b> {
  App::new("init").about("initialize a pidgit directory")
}

pub fn run(matches: &ArgMatches) {
  println!("would run with matches {:#?}", matches);
}
