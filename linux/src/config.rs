use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Serialize, Deserialize, Debug)]
pub struct Config {
    pub ip: String,
    pub commands: HashMap<String, Command>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Command {
    pub help: String,
    pub cmds: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use toml::to_string;

    #[test]
    fn read_config() {
        let toml_str = fs::read_to_string("config.toml").expect("Failed to read config.toml file");
        let config: Result<Config, _> = toml::from_str(&toml_str);
        assert!(config.is_ok());
        //println!("{:#?}", config.unwrap());
    }

    #[test]
    fn write_config() {
        let mut cmds = Vec::<String>::new();
        cmds.push("cmd1".to_owned());
        cmds.push("cmd2".to_owned());
        let mut commands = HashMap::new();
        commands.insert(
            "test_cmd".to_owned(),
            Command {
                help: "help".to_owned(),
                cmds,
            },
        );
        let ip = "192.168.178.170:1234".to_owned();
        let config = Config { ip, commands };
        assert!(to_string(&config).is_ok());
    }
}
