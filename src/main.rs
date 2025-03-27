use epiano_vr_server::{disconnect, join, ReceiveEventBinary, SendEventBinary, ID, SERVER};
use rand::Rng;
use std::{collections::HashMap, mem::size_of, sync::Arc};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{
        tcp::{OwnedReadHalf, OwnedWriteHalf},
        TcpListener,
    },
    sync::{
        mpsc::{self, Receiver, Sender, UnboundedReceiver},
        Mutex,
    },
};

async fn main_send(
    mut event_rx: Receiver<SendEventBinary>,
    clients_for_main: Arc<Mutex<HashMap<ID, mpsc::UnboundedSender<SendEventBinary>>>>,
) {
    while let Some(event) = event_rx.recv().await {
        let clients = clients_for_main.lock().await;

        for (_id, tx) in clients.iter() {
            if let Err(err) = tx.send(event) {
                // ここ unwrap() は大丈夫？
                println!("--main_send: error:{err:?}")
            }
        }
    }
}

// writer か rx が失敗 => return
async fn client_write(mut rx: UnboundedReceiver<SendEventBinary>, mut writer: OwnedWriteHalf) {
    while let Some(event) = rx.recv().await {
        let res = writer.write_all(&event).await;
        if res.is_err() {
            return;
        }
    }
    // clients を取得しておいて、 lock して ドロップするようにすれば、 tx が clients から消えた後に rx がドロップされる => main_send() の tx.send() は unwrap() しても安全！
    // TODO これをやる？
}

// read reader が失敗 => return
async fn client_read(client_id: ID, event_tx: Sender<SendEventBinary>, mut reader: OwnedReadHalf) {
    let should_not_join = join(client_id);
    let should_notdisconnect = disconnect(client_id);

    // msg を buffer として確保しておき、 user_id も書き込んでおく。
    let mut msg = join(client_id);

    while let Ok(_ok) = reader.read_exact(&mut msg[size_of::<ID>()..]).await {
        // 変なイベントははじく。
        if msg != should_not_join && msg != should_notdisconnect {
            // ここの unwrap() は大丈夫？
            // main_send が動き続けているなら、 event_tx は有効 => main_send が動いている機関の方が client_read が動いている期間より長いので .unwrap() は安全。
            event_tx.send(msg).await.unwrap();
        }
    }
}

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
    let (event_tx, event_rx) = mpsc::channel::<SendEventBinary>(100);
    // channels(main -> client)
    let clients = Arc::new(Mutex::new(HashMap::<
        ID,
        mpsc::UnboundedSender<SendEventBinary>,
    >::new()));

    let clients_for_main = Arc::clone(&clients);
    // main loop (event 中継)
    let _main_handle = tokio::spawn(main_send(event_rx, clients_for_main));

    loop {
        let (stream, addr) = listener.accept().await?;
        println!("connected {addr}");
        // channels(main -> client) を持っておく
        let clients = clients.clone();

        // channnel(main -> client)
        let (tx, rx) = mpsc::unbounded_channel::<SendEventBinary>();

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
            println!("start {client_id}");
            let (reader, mut writer) = stream.into_split();

            // 1. まずはつないできた client に id を通知する（かならず待つ）
            let _ = writer.write_all(&client_id.to_le_bytes()).await;

            // 2. 他のクライアントに join の通知をする（これも待つ）
            let _ = event_tx.send(join(client_id)).await;

            // クライアントへの書き込み用の処理
            let write_handle = tokio::spawn(client_write(rx, writer));

            // クライアントからの読み込み用の処理
            let read_handle = tokio::spawn(client_read(
                client_id,
                event_tx.clone(), /* ここで clone をしないと、下の行で event_tx が使えない。 */
                reader,
            ));

            // join する
            tokio::select! {
                _ = write_handle => {
                    println!("write end {client_id}");
                }
                _ = read_handle => {
                    println!("read end {client_id}");
                }
            };

            // 3. read か write ができなくなる => TCP が disconnect => 他のクライアントに disconnect を通知する。
            let _ = event_tx.send(disconnect(client_id)).await;

            // 4. TcpStream が終わったので、 client_id に対応する tx を解放する。
            let mut clients = clients.lock().await;
            clients.remove(&client_id);
            println!("free {client_id}")
        });
    }
}
