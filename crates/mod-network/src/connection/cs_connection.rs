use tokio::net::TcpStream;
use tokio::io::{self, AsyncReadExt, AsyncWriteExt, BufReader, AsyncBufReadExt};
use std::net::SocketAddr;

#[tokio::main]
async fn main() -> io::Result<()> {
    println!("Enter server IP: ");
    let mut server_ip = String::new();
    std::io::stdin().read_line(&mut server_ip)?;
    let server_ip = server_ip.trim();

    let addr: SocketAddr = format!("{}:7878", server_ip).parse().unwrap();
    let stream = TcpStream::connect(&addr).await?;
    
    println!("Connected to server at {}", addr);

    let (mut reader, mut writer) = stream.into_split();
    let (tx, mut rx) = mpsc::channel(100);

    tokio::spawn(async move {
        let mut stdin = BufReader::new(tokio::io::stdin()).lines();
        while let Some(line) = stdin.next_line().await.unwrap() {
            tx.send(line).await.unwrap();
        }
    });

    tokio::spawn(async move {
        while let Some(line) = rx.recv().await {
            writer.write_all(line.as_bytes()).await.unwrap();
        }
    });

    let mut buf = [0; 1024];
    loop {
        let n = reader.read(&mut buf).await?;
        if n == 0 {
            break;
        }

        let response = String::from_utf8_lossy(&buf[..n]); // packet 형식으로 수정 필요. 현재는 임의로 string 읽어들임
        //println!("Updated positions:\n{}", response);
    }

    Ok(())
}