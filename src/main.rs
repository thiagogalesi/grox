extern crate regex;
extern crate getopts;

use std::env;
use std::io;
use std::fs;
use std::iter;
use std::path;
use std::string::String;
use std::collections;
use std::io::prelude::*;
use std::io::BufReader;
use std::fs::File;

use regex::Regex;
use getopts::Options;

macro_rules! println_stderr(
    ($($arg:tt)*) => { {
        let r = writeln!(&mut ::std::io::stderr(), $($arg)*);
        r.expect("failed printing to stderr");
    } }
);

struct FileRegex<'a> {
    frgx: &'a Option<Regex>,
    fnrgx: &'a Option<Regex>,
}

impl<'a> Copy for FileRegex<'a> {}
impl<'a> Clone for FileRegex<'a> {
    fn clone(&self) -> FileRegex<'a> {
        *self
    }
}

struct TextRegex<'a> {
    e: &'a Regex,
    ne: &'a Option<Regex>,
}

impl<'a> Copy for TextRegex<'a> {}
impl<'a> Clone for TextRegex<'a> {
    fn clone(&self) -> TextRegex<'a> {
        *self
    }
}

fn grep(p: &path::Path, tr: TextRegex) {
    let f;
    match File::open(p) {
        Ok(fo) => { f = Some(fo); }
        Err(err) => { println_stderr!("Error opening {}, {}", p.display(), err); return; }
    }
    let f = BufReader::new(f.unwrap());
    let mut ln = 0;
    for line in f.lines() {
        if line.is_ok() {
            let lineok = line.unwrap().to_string();
            let mut print_line = true;
            if tr.e.is_match(&lineok) {
                if tr.ne.is_some() {
                    if tr.ne.clone().unwrap().is_match(&lineok) {
                        print_line = false;
                    }
                }
            } else {
                print_line = false;
            }
            if print_line {
                println!("{} +{} |{}", p.display(), ln, lineok);
            }
        }
        ln += 1;
    }
    // println!("File {} has {} lines",p.display(), ln);
}

fn path_matches(e: &fs::DirEntry, fr: FileRegex) -> bool {
    if fr.frgx.is_some() {
        let entry_name = String::from(e.path().to_str().unwrap());
        let frgxu = fr.frgx.clone().unwrap();
        if frgxu.is_match(&entry_name) {
            if fr.fnrgx.is_some() {
                let fnrgxu = fr.fnrgx.clone().unwrap();
                return !fnrgxu.is_match(&entry_name);
            } else {
                return true;
            }
        } else {
            return false;
        }
    } else {
        return true;
    }
}

fn walk(p: &path::Path, tr: TextRegex, fr: FileRegex) {
    if p.is_dir() {
        let entries_w = fs::read_dir(p);
        let entries: fs::ReadDir;
        match entries_w {
            Ok(e) => { entries = e; }
            Err(err) => {
                println_stderr!("Cannot list path: {}, {}", p.display(), err);
                return;
            }
        }
        for entry in entries {
            let entry = entry.unwrap();
            let fsm = fs::metadata(entry.path());
            let fsp;
            match fsm {
                Ok(fp) => { fsp = fp; }
                Err(err) => {
                    println_stderr!("Cannot stat path: {}, {}",entry.path().display(), err);
                    continue;
                }
            }
            if fsp.is_dir() {
                walk(&entry.path(), tr, fr);
            } else {
                if path_matches(&entry, fr) {
                    grep(&entry.path(), tr);
                }
            }
        }
    } else {
        grep(p, tr);
    }
}

fn print_usage(program: &str, opts: Options) {
    let brief = format!("Usage: {} FILE [options]", program);
    print!("{}", opts.usage(&brief));
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let mut opts = Options::new();
    let program = args[0].clone();
    opts.optopt("e", "", "regex for searching", "RE");
    opts.optopt("", "fx", "shortcut for searching for extensions", "RE");
    opts.optopt("", "frgx", "regex for matching files", "RE");
    opts.optopt("", "fnrgx", "regex for excluding files", "RE");
    opts.optopt("", "ne", "exclude this regex from search", "RE");
    opts.optflag("h", "help", "print this help menu");
    let matches = match opts.parse(&args[1..]) {
        Ok(m) => m,
        Err(f) => panic!(f.to_string()),
    };
    if matches.opt_present("h") {
        print_usage(&program, opts);
        return;
    }
    let frgx_e = matches.opts_str(&["frgx".to_string()]);
    let mut frgx = frgx_e.clone();
    let mut frgx_re = frgx_e.map(|fx| Regex::new(&fx).unwrap());

    let fx_e = matches.opts_str(&["fx".to_string()]);
    if fx_e.is_some() {
        let frgx_fx = format!(r"\.{}$", fx_e.unwrap());
        let re1 = Regex::new(&frgx_fx);
        frgx = Some(frgx_fx); // allows printing below
        frgx_re = Some(re1.unwrap());
    }
    if frgx_re.is_some() {
        println!("FRGX = {}", frgx.clone().unwrap());
    }

    let fnrgx_e = matches.opts_str(&["fnrgx".to_string()]);
    let fnrgx_re = fnrgx_e.map(|fx| Regex::new(&fx).unwrap());
    let e_re_opt = matches.opt_str("e");
    let e_re = e_re_opt.map(|e_rex| Regex::new(&e_rex).unwrap());
    let re = e_re.unwrap();
    let ne_re_opt = matches.opt_str("ne");
    let ne_re = ne_re_opt.map(|ne_rex| Regex::new(&ne_rex).unwrap());

    for a in matches.free {
        let f = path::Path::new(&a);
        let fxr = FileRegex {
            frgx: &frgx_re,
            fnrgx: &fnrgx_re,
        };
        let txr = TextRegex {
            e: &re,
            ne: &ne_re,
        };
        walk(f, txr, fxr);
    }

}
