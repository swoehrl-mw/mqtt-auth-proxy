use mqtt5_protocol::packet::connect::ConnectPacket;

use crate::config::AuthMethod;

pub fn handle_auth_packet(auth: AuthMethod, connect: &mut ConnectPacket) {
    match auth {
        AuthMethod::BasicAuth { username, password } => {
            connect.username = Some(username);
            connect.password = Some(password.as_bytes().to_owned());
        }
        AuthMethod::Jwt { token_file } => {
            let token = std::fs::read_to_string(&token_file).unwrap();
            connect.username = Some("jwt".to_owned());
            connect.password = Some(token.as_bytes().to_owned());
        }
    }
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use mqtt5_protocol::ConnectOptions;
    use tempfile::NamedTempFile;

    use super::*;

    #[test]
    fn test_handle_auth_packet_basicauth() {
        let mut connect = ConnectPacket::new(ConnectOptions::new("clientid"));
        let auth = AuthMethod::BasicAuth {
            username: "admin".to_owned(),
            password: "password".to_owned(),
        };
        handle_auth_packet(auth, &mut connect);
        assert_eq!(connect.username, Some("admin".to_owned()));
        assert_eq!(connect.password, Some("password".as_bytes().to_owned()));
    }
    #[test]
    fn test_handle_auth_packet_jwt() {
        let mut connect = ConnectPacket::new(ConnectOptions::new("clientid"));
        let mut tokenfile = NamedTempFile::new().unwrap();
        let token = "dummytoken".as_bytes();
        tokenfile.write_all(token).unwrap();
        let auth = AuthMethod::Jwt {
            token_file: tokenfile.path().to_path_buf(),
        };
        handle_auth_packet(auth, &mut connect);
        assert_eq!(connect.username, Some("jwt".to_owned()));
        assert_eq!(connect.password, Some(token.to_owned()));
    }
}
