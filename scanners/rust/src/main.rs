use serde::{Deserialize, Serialize};
use std::io::Read;

#[allow(dead_code)]
mod goose {
    include!("goose.rs");

    pub fn scan(command: &str, args: &[String]) -> Option<(String, String)> {
        let ecosystem = command_ecosystem(command)?;
        let (name, _version) = parse_first_package_arg(ecosystem, args)?;
        Some((ecosystem.to_string(), name))
    }
}

#[allow(dead_code)]
mod ramparts {
    include!("ramparts.rs");

    pub fn scan(command: &str, args: &[String]) -> Option<(String, String)> {
        let spec = parse_package_spec_from_command(command, args)?;
        Some((spec.ecosystem.to_string(), spec.name))
    }
}

#[allow(dead_code)]
mod kasetto {
    include!("kasetto.rs");

    pub fn scan(command: &str, args: &[String]) -> Option<(String, String)> {
        let spec = ServerSpec {
            command: Some(command.to_string()),
            args: args.to_vec(),
            ..Default::default()
        };
        let package = launched_package(&spec)?;
        Some((package.ecosystem.osv_name().to_string(), package.name))
    }
}

#[derive(Deserialize)]
struct Case {
    id: String,
    command: String,
    args: Vec<String>,
}

#[derive(Serialize)]
struct Checked {
    ecosystem: String,
    name: String,
}

#[derive(Serialize)]
struct Verdict<'a> {
    id: &'a str,
    scanner: &'a str,
    package: Option<Checked>,
}

fn main() {
    let mut input = String::new();
    std::io::stdin().read_to_string(&mut input).expect("read stdin");
    let cases: Vec<Case> = serde_json::from_str(&input).expect("cases.json is an array of {id, command, args}");
    let scanners: [(&str, fn(&str, &[String]) -> Option<(String, String)>); 3] = [
        ("goose", goose::scan),
        ("ramparts", ramparts::scan),
        ("kasetto", kasetto::scan),
    ];
    for case in &cases {
        for (scanner, scan) in scanners {
            let package = scan(&case.command, &case.args).map(|(ecosystem, name)| Checked { ecosystem, name });
            let verdict = Verdict { id: &case.id, scanner, package };
            println!("{}", serde_json::to_string(&verdict).unwrap());
        }
    }
}
