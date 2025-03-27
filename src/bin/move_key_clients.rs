use epiano_vr_server::{default_s, ReceiveEventBinary, SendEventBinary, ID, SERVER};
use std::{mem::size_of, time::Duration};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
    time::sleep,
};

fn move_msg() -> ReceiveEventBinary {
    // moveEvent (1.0, 1.0, 1.0) にいて (0, 0, 1) を向いてる
    let mut move_msg: ReceiveEventBinary = ReceiveEventBinary::default();

    // eventkind = 2 (Move)
    move_msg[0] = 2;

    // pos_x, pos_y, pos_z = (1.0, 1.0, 1.0)
    move_msg[1..5].copy_from_slice(&1.0f32.to_le_bytes());
    move_msg[5..9].copy_from_slice(&1.0f32.to_le_bytes());
    move_msg[9..13].copy_from_slice(&1.0f32.to_le_bytes());

    // ori_x, ori_y, ori_z = (0.0, 0.0, 90.0) （真横を向くイメージ）
    move_msg[13..17].copy_from_slice(&0.0f32.to_le_bytes());
    move_msg[17..21].copy_from_slice(&0.0f32.to_le_bytes());
    move_msg[21..25].copy_from_slice(&90.0f32.to_le_bytes());

    move_msg
}

fn key_msg() -> ReceiveEventBinary {
    let mut key_msg: ReceiveEventBinary = ReceiveEventBinary::default();
    let note_number: u16 = 69;
    key_msg[0] = 3;
    key_msg[1..3].copy_from_slice(&note_number.to_le_bytes());
    key_msg
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("start");
    let time: usize = 1000;

    let stream = TcpStream::connect(&SERVER).await?;
    println!("connected to {SERVER}");

    let (mut reader, mut writer) = stream.into_split();

    let mut usr: [u8; 4] = [0u8; size_of::<ID>()];
    let _ = reader.read_exact(&mut usr).await;
    let usr = u32::from_le_bytes(usr);
    println!("given id: {:X}", usr);

    tokio::spawn(async move {
        let mut get_msg = default_s();
        loop {
            match reader.read_exact(&mut get_msg).await {
                Ok(ok) => {
                    println!("return {ok}: {}", hex::encode(get_msg));
                }
                Err(err) => {
                    println!("err: {err}");
                    break;
                }
            };
        }
    });

    let mut angle: f32 = 0_f32;

    let mut move_msg = move_msg();
    let mut key_msg = key_msg();
    let mut on = true;

    // 送信タスク
    loop {
        move_msg[13..17].copy_from_slice(&0.0f32.to_le_bytes());
        move_msg[17..21].copy_from_slice(&angle.to_le_bytes());
        move_msg[21..25].copy_from_slice(&0.0f32.to_le_bytes());

        let velocity: u16 = if on { 100 } else { 0 };
        key_msg[3..5].copy_from_slice(&velocity.to_le_bytes());

        writer.write_all(&move_msg).await?;
        writer.write_all(&key_msg).await?;

        // 次のフレーム用に回転を更新（0〜360°ループ）
        angle += 5.0;
        if angle >= 360.0 {
            angle -= 360.0;
        }
        on = !on;
        sleep(Duration::from_millis(time as u64)).await;
    }
}
