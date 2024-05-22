use regex::Regex;
use tokio;

#[macro_export]
#[macro_use]
macro_rules! call_function {
    ($func:expr) => {
        async { $func().await }
    };

    ($func:expr, $args:expr, single) => {
        async {
            let re = Regex::new(r#""[^"]+":\s*"([^"]+)""#).expect("Failed to parse arguments");
            let arg = re
                .captures_iter($args)
                .filter_map(|cap| cap.get(1).map(|m| m.as_str()))
                .next()
                .expect("No argument found");

            $func(arg).await
        }
    };

    ($func:expr, $args:expr, multi) => {
        async {
            let re = Regex::new(r#""[^"]+":\s*"([^"]+)""#).expect("Failed to parse arguments");
            let args: Vec<&str> = re
                .captures_iter($args)
                .filter_map(|cap| cap.get(1).map(|m| m.as_str()))
                .collect();

            $func(&args).await
        }
    };
}

/* macro_rules! call_function {
    ($func:expr, $args:expr) => {
        async {
            let re = Regex::new(r#""[^"]+":\s*"([^"]+)""#).expect("failed to parse arguments");
            let args: Vec<&str> = re
                .captures_iter($args)
                .filter_map(|cap| cap.get(1).map(|m| m.as_str()))
                .collect();
            match args.len() {
                0 => panic!(),
                1 => $func(args[0]).await, // Pass a single &str if there's only one
                _ => $func(&args.join(", ")).await, // Pass a joined string if there are multiple
            }
        }
    };
}


enum ArgTuple {
    None,
    Single(&'static str),
    Multiple(Vec<&'static str>),
}

macro_rules! call_function {
    ($func:ident, $args_raw:expr) => {
        async {
            let re = Regex::new(r#""([^"]+)":\s*"([^"]+)""#).expect("Failed to compile regex");
            let mut arg_tuple = ArgTuple::None;
            let args: Vec<&str> = re
                .captures_iter($args_raw)
                .filter_map(|cap| cap.get(1).map(|m| m.as_str()))
                .collect();
            match args.len() {
                0 => arg_tuple = ArgTuple::None,
                1 => arg_tuple = ArgTuple::Single(args[0].clone()),
                _ => arg_tuple = ArgTuple::Multiple(args),
            }
            match arg_tuple {
                ArgTuple::None => $func().await,
                ArgTuple::Single( arg) => $func(arg).await,
                ArgTuple::Multiple(ref args) => $func(&args.join(", ")).await,
            }
        }
    };
}


macro_rules! call_function {
    ($func:expr) => {
        async {
            $func().await
        }
    };

    ($func:expr, $($args:expr),*) => {
        async {
            let re = Regex::new(r#""[^"]+":\s*"([^"]+)""#).expect("Failed to parse regex");
            let mut args_vec = Vec::new();

            $(
                let captures = re.captures_iter($args)
                    .filter_map(|cap| cap.get(1).map(|m| m.as_str()))
                    .collect::<Vec<&str>>();

                if captures.is_empty() {
                    return Err(Error::msg("No valid arguments found"));
                }
                args_vec.extend(captures);
            )*

            match args_vec.len() {
                0 => Err(Error::msg("No arguments provided")),
                1 => $func(args_vec[0]).await,
                _ => $func(&args_vec[..]).await,
            }
        }
    };
}

 */