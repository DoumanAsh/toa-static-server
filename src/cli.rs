extern crate clap;

use self::clap::{App, Arg, ArgMatches};

const NAME: &'static str = env!("CARGO_PKG_NAME");
const AUTHOR: &'static str = env!("CARGO_PKG_AUTHORS");
const VERSION: &'static str = env!("CARGO_PKG_VERSION");
const ABOUT: &'static str = "
Kawaii Static HTTP Server.";

pub fn parse() -> ArgMatches<'static> {
    App::new(NAME).about(ABOUT)
                  .author(AUTHOR)
                  .version(VERSION)
                  .arg(Arg::with_name("dir").default_value(".")
                                            .takes_value(true)
                                            .help("Directory to serve. Current directory used by default"))
                  .arg(Arg::with_name("port").short("p")
                                             .takes_value(true)
                                             .default_value("13666")
                                             .help("Specifies port number to use"))
                  .get_matches()
}

///Wrapper over clap::ArgMatches
pub struct Args(pub ArgMatches<'static>);

impl Args {
    ///Parses commandline arguments and creates new instance
    pub fn new() -> Args {
        Args(parse())
    }

    ///Get unconditionally value. Panics on no value.
    pub fn get_string(&self, key: &str) -> String {
        self.0.value_of(key).unwrap().to_string()
    }

    ///Get unconditionally u16. Panics on no value.
    pub fn get_u16(&self, key: &str) -> Result<u16, String> {
        use std::str::FromStr;

        let value = self.0.value_of(key).unwrap();
        u16::from_str(value).map_err(|error| format!("Invalid {}: '{}'. Error: {}", key, value, error))
    }
}
