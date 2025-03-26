use epiano_vr_server::{default_s, ReceiveEventBinary, SendEventBinary, ID, SERVER};
use std::{mem::size_of, time::Duration};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
    time::sleep,
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("start");
    let time: usize = 1000;

    let mut send_msg = ReceiveEventBinary::default();
    let msg_str = "hello world!";
    send_msg[0..msg_str.len()].clone_from_slice(msg_str.as_bytes());
    println!("{}", hex::encode(send_msg));

    let mut get_msg = default_s();

    let stream = TcpStream::connect(&SERVER).await?;
    println!("connected to {SERVER}");

    let (mut reader, mut writer) = stream.into_split();

    let mut usr: [u8; 4] = [0u8; size_of::<ID>()];
    let _ = reader.read_exact(&mut usr).await;
    let usr = u32::from_le_bytes(usr);
    println!("given id: {:X}", usr);

    tokio::spawn(async move {
        loop {
            match reader.read_exact(&mut get_msg).await {
                Ok(ok) => {
                    println!("return {ok}: {}", hex::encode(get_msg));
                    // FIX unwrap
                }
                Err(err) => {
                    println!("err: {err}");
                    break;
                }
            };
        }
    });

    // 送信タスク
    loop {
        writer.write_all(&send_msg).await?;
        println!("write");

        sleep(Duration::from_millis(time as u64)).await;
    }
}
