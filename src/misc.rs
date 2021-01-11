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
