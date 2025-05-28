use local_ip_address::local_ip;
use std::{default::Default, str::FromStr};

/// ipv4 only
pub struct Addr {
    ip: String,
    port: u16,
}

impl Default for Addr {
    fn default() -> Self {
        Self {
            ip: "localhost".to_string(),
            port: 7878,
        }
    }
}

impl ToString for Addr {
    fn to_string(&self) -> String {
        format!("{}:{}", self.ip, self.port)
    }
}

impl FromStr for Addr {
    type Err = String;

    fn from_str(addr: &str) -> Result<Self, Self::Err> {
        if addr.trim().len() == 0 {
            return Ok(Self::default());
        }

        let usage_msg = "address format: <mode(or ip)>:<port>\n";
        let public_ip = local_ip().unwrap().to_string();
        let mode_list_msg = format!(
            "mode: \n  - localhost\n  - public \t\t(ip address: {})\n",
            public_ip
        );

        // port를 분리. ipv6은 신경쓰지 않는다.
        let mut args = addr.split(":").map(|s| s.trim());

        let mode = args.next().unwrap();

        let ip = match mode {
            "public" => public_ip,
            _ if mode == &public_ip => mode.to_string(),

            "localhost" | "127.0.0.1" => mode.to_string(),

            _ => {
                // let help_message = format!("invalid mode(or ip): '{}'\n{}", mode, mode_list_msg);
                // return Err(help_message);
                mode.to_string()
            }
        };

        let addr = match args.next() {
            Some(port) => match port.parse::<u16>() {
                Ok(port) => Self { ip, port },
                Err(_) => {
                    let help_message = format!("invalid port number: '{}'.\n  port number must be a 16-bit unsigned integer.", port);
                    return Err(help_message);
                }
            },
            None => Self {
                ip,
                ..Self::default()
            },
        };

        let remained = args.collect::<Vec<_>>();

        // 잘못된 입력이 있을 경우
        if remained.len() > 0 {
            let help_message = format!(
                "too many arguments: {:?}\n{}{}",
                remained, usage_msg, mode_list_msg
            );
            return Err(help_message);
        }

        Ok(addr)
    }
}
