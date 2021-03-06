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
    ctx: u8,
}

impl<'a> Copy for TextRegex<'a> {}
impl<'a> Clone for TextRegex<'a> {
    fn clone(&self) -> TextRegex<'a> {
        *self
    }
}

fn grep_file(p: &path::Path, tr: TextRegex) {
    let mut use_ctx: bool = false;
    // if the context is 1 line, we need to store 2 lines
    // hence the +1
    let rbuff_size:usize = (tr.ctx+1).into();
    let mut rbuff: Vec<String> = Vec::with_capacity(rbuff_size.into());
    if tr.ctx != 0 {
        use_ctx = true;
    }
    let f_ = match File::open(p) {
        Ok(fo) => fo,
        Err(err) => { println_stderr!("Error opening {}, {}", p.display(), err); return; }
    };
    let f = BufReader::new(f_);
    let mut ln:usize = 0;
    let mut print_line;
    let mut bpp:usize = 0;
    let mut bpa:usize = 0;
    for line_m in f.lines() {
        if !line_m.is_ok() {
            continue;
        }
        let line_s = line_m.unwrap().to_string();
        if ln < rbuff_size {
            rbuff.push(line_s.clone())
        } else if rbuff_size > 0 {
            let idx:usize = (ln%rbuff_size).into();
            rbuff[idx] = line_s.clone();
        }
        let lineok = line_s.clone();
        print_line = grep(&lineok, &tr);
        if print_line {
            if !use_ctx {
                println!("{} +{} |{}", p.display(), ln, lineok);
            } else {
                // print the lines before
                println!("--");
                let act_rbsize = rbuff.len();
                for x in 0..act_rbsize {
                    // you want to start on the first line, which is
                    // the next line on the buffer
                    bpp = (1+ln+x)%act_rbsize;
                    let actual_ln = ln - (act_rbsize - x - 1);
                    println!("{} +{} |{}", p.display(), actual_ln, rbuff[bpp]);
                }
            }
        }
        ln += 1;

    }
}

fn grep_stdin(tr: TextRegex) {
    let mut s = String::new();
    let stdin = io::stdin();
    let mut ln = 0;
    let mut print_line;
    while stdin.read_line(&mut s).unwrap() > 0 {
        print_line = grep(&s, &tr);
        s.pop(); // removes newline
        if print_line {
            println!("(standard input) +{} |{}", ln, &s);
        }
        s.clear();
        ln += 1;
    }

}

fn grep(line: &String, tr: &TextRegex) -> bool {
    let mut print_line = true;
    if tr.e.is_match(line) {
        if tr.ne.is_some() {
            if tr.ne.clone().unwrap().is_match(line) {
                print_line = false;
            }
        }
    } else {
        print_line = false;
    }
    print_line
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
        entries = match entries_w {
            Ok(e) => e,
            Err(err) => {
                println_stderr!("Cannot list path: {}, {}", p.display(), err);
                return;
            }
        };
        for entry in entries {
            let entry = entry.unwrap();
            let fsm = fs::metadata(entry.path());
            let fsp = match fsm {
                Ok(fp) => fp,
                Err(err) => {
                    println_stderr!("Cannot stat path: {}, {}",entry.path().display(), err);
                    continue;
                }
            };
            if fsp.is_dir() {
                walk(&entry.path(), tr, fr);
            } else {
                if path_matches(&entry, fr) {
                    grep_file(&entry.path(), tr);
                }
            }
        }
    } else {
        grep_file(p, tr);
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
    opts.optopt("C", "", "lines of context", "LINES");
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

    let ctx_opt = matches.opt_str("C");
    let ctx:u8 = match ctx_opt {
        Some(x) => x.parse::<u8>().unwrap(),
        None => 0,
    };
    
    let frgx_e = matches.opt_str("frgx");
    let mut frgx = frgx_e.clone();
    let mut frgx_re = frgx_e.map(|fx| Regex::new(&fx).unwrap());

    let fx_e = matches.opt_str("fx");
    if fx_e.is_some() {
        let frgx_fx = format!(r"\.{}$", fx_e.unwrap());
        let re1 = Regex::new(&frgx_fx);
        frgx = Some(frgx_fx); // allows printing below
        frgx_re = Some(re1.unwrap());
    }
    if frgx_re.is_some() {
        println!("FRGX = {}", frgx.clone().unwrap());
    }

    let fnrgx_e = matches.opt_str("fnrgx");
    let fnrgx_re = fnrgx_e.map(|fx| Regex::new(&fx).unwrap());
    let e_re_opt = matches.opt_str("e");
    let e_re = e_re_opt.map(|e_rex| Regex::new(&e_rex).unwrap());
    let re = e_re.unwrap();
    let ne_re_opt = matches.opt_str("ne");
    let ne_re = ne_re_opt.map(|ne_rex| Regex::new(&ne_rex).unwrap());

    let mut from_stdin = false;

    let mut free_matches = matches.free;

    if free_matches.len() == 0 {
        free_matches.push(".".to_owned()); 
    }

    let txr = TextRegex {
        e: &re,
        ne: &ne_re,
        ctx: ctx,
    };

    for a in free_matches {
        if a == "--" {
            from_stdin = true;
            break;
        }
        let f = path::Path::new(&a);
        let fxr = FileRegex {
            frgx: &frgx_re,
            fnrgx: &fnrgx_re,
        };
        walk(f, txr, fxr);
    }

    if from_stdin {
        grep_stdin(txr);
    }

}
