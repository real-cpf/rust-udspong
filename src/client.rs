use std::{os::unix::net::UnixStream, io::{Read, Bytes, Write}};

use bytes::BufMut;



fn main(){
    let bind_addr = "/tmp/uds.sock";
    let mut stream = UnixStream::connect(bind_addr).unwrap();
    let mut input = String::new();
    match std::io::stdin().read_line(&mut input) {
        Ok(n)=>{
            println!("{n}")
        },
        Err(e)=>println!("{e}")
    }
    let input = input.trim().to_string().clone();
    let mut l:Vec<u8> = vec![];
    l.put_u16(5_u16);
    let bb = input.as_bytes();
    l.put_u32(bb.len() as u32);
    l.put(bb);
    l.put_u8(b'\r');
    l.put_u8(b'\n');
    
    stream.write_all(&l.to_vec()).unwrap();
    let mut buffer:[u8;1024] = [0; 1024];
    loop {
        stream.read(&mut buffer[..]).unwrap();
        let s = String::from_utf8(buffer.to_vec()).unwrap();
        println!(">:{}",s);
        println!("to:");
        let mut input = String::new();
        match std::io::stdin().read_line(&mut input) {
            Ok(n)=>{
                println!("{n}")
            },
            Err(e)=>println!("{e}")
        }
        let input = input.trim().to_string().clone();
        let to = input.as_bytes();
        println!("say:");
        let mut input = String::new();
        match std::io::stdin().read_line(&mut input) {
            Ok(n)=>{
                println!("{n}")
            },
            Err(e)=>println!("{e}")
        }
        input = input.trim().to_string();
        let msg = input.as_bytes();
        l.clear();
        l.put_u16(4_u16);
        let ln = to.len() + msg.len() + 1;
        l.put_u32(ln as u32);
        l.put(to);
        l.put_u8(b' ');
        l.put(msg);
        
        l.put_u8(b'\r');
        l.put_u8(b'\n');
        stream.write_all(&l.to_vec()).unwrap();
    }
    

}