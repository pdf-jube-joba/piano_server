use epiano_vr_server::{default_s, ReceiveEventBinary, SendEventBinary, SERVER};
use std::time::Instant;
use tokio::{
    io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader},
    net::TcpStream,
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let mut send_msg = ReceiveEventBinary::default();
    let msg_str = "hello world!";
    send_msg[0..msg_str.len()].clone_from_slice(msg_str.as_bytes());

    let mut received_msg = default_s();

    let stream = TcpStream::connect(SERVER).await?;

    let (mut reader, mut writer) = stream.into_split();

    loop {
        let start_time = Instant::now();
        writer.write_all(&send_msg).await;
        writer.flush().await;
        reader.read_exact(&mut received_msg).await;
        let duration = start_time.elapsed();
        println!("RTT: {duration:?}");

        // 少し待って次の送信（例: 1秒ごと）
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
    }
}
