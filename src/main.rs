use std::collections::HashMap;
use std::error::Error;
use std::io::Cursor;

use bytes::Buf;
use bytes::Bytes;
use bytes::BytesMut;
use tokio::io::AsyncReadExt;
use tokio::io::AsyncWriteExt;
use tokio::{
    io::BufWriter,
    net::{UnixListener, UnixStream},
};

#[tokio::main]
async fn main() {
    let listener = UnixListener::bind("/tmp/uds.sock").unwrap();

    let mut channel_map:HashMap<String, UnixStream> = HashMap::new();
    loop {
        let (mut socket, _) = listener.accept().await.unwrap();
        tokio::spawn(async move {
            loop {
                let mut buf = BytesMut::with_capacity(1024);
                if 0 == socket.read_buf(&mut buf).await.unwrap() {
                    break;
                }
                let len = buf.len();
                let arr = buf.to_vec();
                let s = String::from_utf8(arr).unwrap();
                let ss:Vec<&str> = s.split(' ').collect();
                
                println!("key {}",ss[0]);
                socket.write_all(ss[1].as_bytes()).await;
                
            }
        });
        println!("done this accept!");
    }
}
