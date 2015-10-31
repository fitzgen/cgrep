extern crate docopt;
extern crate threadpool;
extern crate rustc_serialize;
extern crate walker;

use docopt::Docopt;
use std::env;
use std::error::Error;
use std::fs;
use std::io;
use std::io::BufRead;
use std::io::Write;
use std::path;
use std::process;

const USAGE: &'static str = "
Usage: cgrep <string> <directory>
";

#[derive(RustcDecodable)]
struct Args {
    arg_string: String,
    arg_directory: String,
}

fn get_directory_walker(dir: &path::Path) -> walker::Walker {
    walker::Walker::new(dir)
        .unwrap_or_else(|err| {
            writeln!(io::stderr(), "Error reading directory: {}", &err.description()).unwrap();
            process::exit(1);
        })
}

fn main() {
    let args: Args = Docopt::new(USAGE)
        .and_then(|d| d.argv(env::args().into_iter()).decode())
        .unwrap_or_else(|e| e.exit());

    let dir_path = path::Path::new(&args.arg_directory);
    let entries = get_directory_walker(dir_path);

    let file_paths = entries
        .filter_map(|entry| entry.map(|e| Some(e))
                                 .unwrap_or_else(|err| {
                                     writeln!(io::stderr(), "Error: {}", err.description()).unwrap();
                                     None
                                 }))
        .filter_map(|entry| entry.file_type()
                                 .map(|file_type| if file_type.is_dir() { None } else { Some(entry) })
                                 .unwrap_or_else(|err| {
                                     writeln!(io::stderr(), "Error: {}", err.description()).unwrap();
                                     None
                                 }))
        .map(|entry| entry.path());

    let files = file_paths
        .map(|path| fs::File::open(path))
        .filter_map(|path| path.map(|f| Some(f))
                               .unwrap_or_else(|err| {
                                   writeln!(io::stderr(), "Error: {}", err.description()).unwrap();
                                   None
                               }));

    let pool = threadpool::ThreadPool::new(8);

    for file in files {
        let pattern = args.arg_string.clone();
        pool.execute(move || {
            for line in io::BufReader::new(file).lines() {
                match line {
                    Err(err) => {
                        writeln!(io::stderr(), "Error: {}", err.description()).unwrap();
                        continue;
                    },
                    Ok(line) => {
                        if line.contains(&pattern) {
                            println!("{}", line);
                        }
                    }
                }
            }
        });
    }
}
