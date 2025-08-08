use crate::Config;

pub fn interpret(mut input: String, config: &Config) -> Vec<(bool, String)> {
    let mut r: Vec<(bool, String)> = Vec::new();

    if config.commands.contains_key(&input) {
        let cmds = config.commands.get(&input).unwrap();
        for cmd in &cmds.cmds {
            let mut cmd = cmd.clone();
            cmd.push('\n');
            r.push((true, cmd));
        }
    } else {
        match input.as_str() {
            "help" => {
                r.push((false, "help".to_owned()));
                for (name, cmd) in &config.commands {
                    let help_str = format!("  {:<10}{}", *name, cmd.help);
                    r.push((false, help_str));
                } 
            }
            _ => {
                if input.len() > 0 {
                    let cmd = format!("${}\n", input.as_str());
                    r.push((true, cmd));
                }
            } 
        }
    }
    r
}