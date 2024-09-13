use std::env;
use local_ip_address::local_ip;


pub fn get_addr() -> Result<(String, u16), String> {
    let usage_msg = "Usage: .exe <mode(or ip)>:<port>\n";
    let public_ip = local_ip().unwrap().to_string();
    let mode_list_msg = format!("mode: \n  - localhost\n  - public \t\t(ip address: {})\n", public_ip);

    let args: Vec<String> = env::args().collect();

    match args.len() {
        // 입력하지 않은 경우(default)
        1 => Ok(("localhost".to_string(), 7878)),

        // 입력한 경우
        2 => {
            let mut arg_iter = args[1].split(":");

            let ip = arg_iter.next().unwrap();
            let ip = match ip {
                "public" => &public_ip,
                _ if ip == &public_ip => ip,

                "localhost" | "127.0.0.1" => ip,

                _ => {
                    let help_message = format!("invalid mode(or ip): '{}'\n{}", ip, mode_list_msg);
                    return Err(help_message)
                }
            };
            
            let port = match arg_iter.next() {
                Some(port) => {
                    match port.parse::<u16>() {
                        Ok(port) => port,
                        Err(_) => {
                            let help_message = format!("invalid port number: '{}'. port number must be 16bit unsigned integer.", port);
                            return Err(help_message)
                        }
                    }
                },
                None => 7878,
            };
        
            Ok((ip.to_string(), port))
        },

        // 잘못된 입력
        _ => {
            let help_message = format!("invalid arguments: {:?}\n{}{}", args[1..].to_vec(), usage_msg, mode_list_msg);
            Err(help_message)
        }
    }
}