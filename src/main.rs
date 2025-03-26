use epiano_vr_server::{
    default_s, disconnect, join, ReceiveEventBinary, SendEventBinary, ID, SERVER,
};
use rand::Rng;
use std::{collections::HashMap, mem::size_of, sync::Arc};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
    sync::{mpsc, Mutex},
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!(
        "eventlen: {}, userlen: {}",
        size_of::<ReceiveEventBinary>(),
        size_of::<ID>()
    );

    let mut rng = rand::rng();

    let listener = TcpListener::bind(&SERVER)
        .await
        .expect("error at Server starts");
    println!("Server starts at {SERVER}");

    // channel(client -> main)
    let (event_tx, mut event_rx) = mpsc::channel::<SendEventBinary>(100);
    // channels(main -> client)
    let clients = Arc::new(Mutex::new(HashMap::<
        ID,
        mpsc::UnboundedSender<SendEventBinary>,
    >::new()));

    let clients_for_main = Arc::clone(&clients);
    // main loop (event 中継)
    tokio::spawn(async move {
        while let Some(event) = event_rx.recv().await {
            let clients = clients_for_main.lock().await;

            for (_id, tx) in clients.iter() {
                let _ = tx.send(event);
            }
        }
    });

    loop {
        let (stream, addr) = listener.accept().await?;
        println!("connected {addr}");
        // channels(main -> client) を持っておく
        let clients = clients.clone();

        // channnel(main -> client)
        let (tx, mut rx) = mpsc::unbounded_channel::<SendEventBinary>();

        // channnel(main -> client) を clients に登録する（idをかぶらないように）
        let client_id = {
            // ロックをできる限り早く解放
            let mut clients = clients.lock().await;
            // ほとんどかぶらないと思うけど、かぶったら乱数引き直しのため
            loop {
                let client_id: u32 = rng.random();
                if clients.get(&client_id).is_none() {
                    clients.insert(client_id, tx);
                    break client_id;
                }
            }
        };
        println!("assign {addr} -> {:X}", client_id);

        // sender(client -> main) を渡す用
        let event_tx = event_tx.clone();
        // 各 client での読み込み/書き込み処理
        tokio::spawn(async move {
            // client の stream を読む用の buf だが、そのまま event として送り出したい（使いまわす）。
            let mut msg = default_s();
            // 自分の client_id は確定しているので埋めておく。
            msg[0..size_of::<ID>()].clone_from_slice(&client_id.to_le_bytes());

            let (mut reader, mut writer) = stream.into_split();
            // client(stream) -> msg と main -> client(stream) の少なくともどちらかを tokio::spawn(async move {}) に入れる必要がある。
            // ここでは main -> client(stream) を選んだ（AIのおすすめ）。基本的にはどちらでもよさそう。

            // 1. まずは client に id を通知する（かならず待つ）
            // client_id 以外は 0 fill されているのが Join
            let _ = writer.write_all(&msg[0..size_of::<ID>()]).await;

            // 2. これは待たなくていいけど、とりあえず .await にしてしまう。
            let _ = event_tx.send(msg.clone()).await;

            // 書き込み用の処理（非同期に切り出す）
            // (channel(main -> client)) の msg を受け取って stream に書く。
            tokio::spawn(async move {
                while let Some(event) = rx.recv().await {
                    // println!("W: {client_id}...{}", hex::encode(event));

                    // FIX unwrap
                    writer.write_all(&event).await.unwrap();
                }
            });

            // client(stream) -> msg
            // stream の read をして msg を main に流す
            println!("new R: {client_id}");

            loop {
                match reader.read_exact(&mut msg[size_of::<ID>()..]).await {
                    Ok(ok) => {
                        // println!("R: {client_id} with {ok}");
                        // FIX unwrap
                        event_tx.send(msg).await.unwrap();
                    }
                    Err(err) => {
                        // println!("R: {client_id} err: {err}");
                        break;
                    }
                };
            }

            // readerループ終了後

            // 3. 他の参加者に通知する。
            // client_id 以外は 1 fill されているのが退出
            msg[size_of::<ID>()..].clone_from_slice(&[1; size_of::<SendEventBinary>()]);
            let _ = event_tx.send(msg).await;

            // TcpStream が終わったので解放
            {
                let mut clients = clients.lock().await;
                clients.remove(&client_id);
                println!("free {client_id}")
            }
        });
    }
}
