use anyhow::{format_err, Result};
use std::collections::HashMap;

pub fn parse_query_string(s: &str) -> Result<HashMap<String, String>> {
    let mut args: HashMap<String, String> = HashMap::new();

    if s.len() != 0 {
        let args_str: Vec<&str> = s.split(',').collect();
        // Generate arg pairs
        for x in args_str {
            let arg: Vec<&str> = x.split('=').collect();
            if arg.len() != 2 {
                return Err(format_err!("Invalid query string"));
            } else {
                args.insert(arg[0].to_string(), arg[1].to_string());
            }
        }
    }

    Ok(args)
}

// Decoding expire_time
// Format: <time>{m, h, d, w}
pub fn decode_expire_time(t: &str) -> Result<u64> {
    let mut time = t.to_string();
    let expire_time_unit = time.pop();
    let multiplier = match expire_time_unit {
        None => -1,
        Some(char) => match char {
            'm' => 1,
            'h' => 60,
            'd' => 60 * 24,
            _ => -1,
        },
    };

    if multiplier != -1 || time.parse::<i64>().is_ok() {
        let expire_time = time.parse::<i64>().unwrap() * multiplier;
        if expire_time > 0 {
            return Ok(expire_time as u64);
        }
    }

    Err(format_err!("Invalid expire time."))
}

use actix_web::{web, HttpRequest, HttpResponse, Responder};
pub fn return_500() -> impl Responder {
    HttpResponse::InternalServerError().body("Internal Server Error")
}

use syntect::highlighting::{Color, ThemeSet};
use syntect::html::highlighted_html_for_string;
use syntect::parsing::SyntaxSet;

pub fn get_highlighted_html(string: &str, ext: &str) -> Result<String> {
    let ss = SyntaxSet::load_defaults_newlines();
    let sr = match ss.find_syntax_by_extension(ext) {
        Some(sr) => sr,
        None => {
            return Err(format_err!(""));
        }
    };
    let ts = ThemeSet::load_defaults();

    let theme = &ts.themes["base16-ocean.dark"];
    let html = highlighted_html_for_string(string, &ss, sr, theme);
    Ok(html)
}

#[macro_export]
macro_rules! skip_fail {
    ($res:expr) => {
        match $res {
            Ok(val) => val,
            Err(e) => {
                error!("Error: {}; skipped.", e);
                continue;
            }
        }
    };
}

#[macro_export]
macro_rules! ise_on_err { // Produce HTTP 500 on Err(sth)
    ($res:expr) => {
        match $res {
            Ok(val) => val,
            Err(e) => {
                use log::error;
                error!("{}", e.to_string());
                return HttpResponse::InternalServerError().body("Internal Server Error");
            }
        }
    };
}
