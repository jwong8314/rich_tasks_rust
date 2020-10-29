#![warn(rust_2018_idioms)]
#![cfg(feature = "full")]

use futures::future::poll_fn;
use std::sync::Arc;
use tokio::{io::ReadBuf, net::UdpSocket};

const MSG: &[u8] = b"hello";
const MSG_LEN: usize = MSG.len();

#[tokio::test]
async fn send_recv() -> std::io::Result<()> {
    let sender = UdpSocket::bind("127.0.0.1:0").await?;
    let receiver = UdpSocket::bind("127.0.0.1:0").await?;

    sender.connect(receiver.local_addr()?).await?;
    receiver.connect(sender.local_addr()?).await?;

    sender.send(MSG).await?;

    let mut recv_buf = [0u8; 32];
    let len = receiver.recv(&mut recv_buf[..]).await?;

    assert_eq!(&recv_buf[..len], MSG);
    Ok(())
}

#[tokio::test]
async fn send_recv_poll() -> std::io::Result<()> {
    let sender = UdpSocket::bind("127.0.0.1:0").await?;
    let receiver = UdpSocket::bind("127.0.0.1:0").await?;

    sender.connect(receiver.local_addr()?).await?;
    receiver.connect(sender.local_addr()?).await?;

    poll_fn(|cx| sender.poll_send(cx, MSG)).await?;

    let mut recv_buf = [0u8; 32];
    let mut read = ReadBuf::new(&mut recv_buf);
    let _len = poll_fn(|cx| receiver.poll_recv(cx, &mut read)).await?;

    assert_eq!(read.filled(), MSG);
    Ok(())
}

#[tokio::test]
async fn send_to_recv_from() -> std::io::Result<()> {
    let sender = UdpSocket::bind("127.0.0.1:0").await?;
    let receiver = UdpSocket::bind("127.0.0.1:0").await?;

    let receiver_addr = receiver.local_addr()?;
    sender.send_to(MSG, &receiver_addr).await?;

    let mut recv_buf = [0u8; 32];
    let (len, addr) = receiver.recv_from(&mut recv_buf[..]).await?;

    assert_eq!(&recv_buf[..len], MSG);
    assert_eq!(addr, sender.local_addr()?);
    Ok(())
}

#[tokio::test]
async fn send_to_recv_from_poll() -> std::io::Result<()> {
    let sender = UdpSocket::bind("127.0.0.1:0").await?;
    let receiver = UdpSocket::bind("127.0.0.1:0").await?;

    let receiver_addr = receiver.local_addr()?;
    poll_fn(|cx| sender.poll_send_to(cx, MSG, &receiver_addr)).await?;

    let mut recv_buf = [0u8; 32];
    let mut read = ReadBuf::new(&mut recv_buf);
    let addr = poll_fn(|cx| receiver.poll_recv_from(cx, &mut read)).await?;

    assert_eq!(read.filled(), MSG);
    assert_eq!(addr, sender.local_addr()?);
    Ok(())
}

#[tokio::test]
async fn send_to_peek_from() -> std::io::Result<()> {
    let sender = UdpSocket::bind("127.0.0.1:0").await?;
    let receiver = UdpSocket::bind("127.0.0.1:0").await?;

    let receiver_addr = receiver.local_addr()?;
    poll_fn(|cx| sender.poll_send_to(cx, MSG, &receiver_addr)).await?;

    // peek
    let mut recv_buf = [0u8; 32];
    let (n, addr) = receiver.peek_from(&mut recv_buf).await?;
    assert_eq!(&recv_buf[..n], MSG);
    assert_eq!(addr, sender.local_addr()?);

    // peek
    let mut recv_buf = [0u8; 32];
    let (n, addr) = receiver.peek_from(&mut recv_buf).await?;
    assert_eq!(&recv_buf[..n], MSG);
    assert_eq!(addr, sender.local_addr()?);

    let mut recv_buf = [0u8; 32];
    let (n, addr) = receiver.recv_from(&mut recv_buf).await?;
    assert_eq!(&recv_buf[..n], MSG);
    assert_eq!(addr, sender.local_addr()?);

    Ok(())
}

#[tokio::test]
async fn send_to_peek_from_poll() -> std::io::Result<()> {
    let sender = UdpSocket::bind("127.0.0.1:0").await?;
    let receiver = UdpSocket::bind("127.0.0.1:0").await?;

    let receiver_addr = receiver.local_addr()?;
    poll_fn(|cx| sender.poll_send_to(cx, MSG, &receiver_addr)).await?;

    let mut recv_buf = [0u8; 32];
    let mut read = ReadBuf::new(&mut recv_buf);
    let addr = poll_fn(|cx| receiver.poll_peek_from(cx, &mut read)).await?;

    assert_eq!(read.filled(), MSG);
    assert_eq!(addr, sender.local_addr()?);

    let mut recv_buf = [0u8; 32];
    let mut read = ReadBuf::new(&mut recv_buf);
    poll_fn(|cx| receiver.poll_peek_from(cx, &mut read)).await?;

    assert_eq!(read.filled(), MSG);
    let mut recv_buf = [0u8; 32];
    let mut read = ReadBuf::new(&mut recv_buf);

    poll_fn(|cx| receiver.poll_recv_from(cx, &mut read)).await?;
    assert_eq!(read.filled(), MSG);
    Ok(())
}

#[tokio::test]
async fn split() -> std::io::Result<()> {
    let socket = UdpSocket::bind("127.0.0.1:0").await?;
    let s = Arc::new(socket);
    let r = s.clone();

    let addr = s.local_addr()?;
    tokio::spawn_with_priority(async move {
        s.send_to(MSG, &addr).await.unwrap();
    }, 3);
    let mut recv_buf = [0u8; 32];
    let (len, _) = r.recv_from(&mut recv_buf[..]).await?;
    assert_eq!(&recv_buf[..len], MSG);
    Ok(())
}

#[tokio::test]
async fn split_chan() -> std::io::Result<()> {
    // setup UdpSocket that will echo all sent items
    let socket = UdpSocket::bind("127.0.0.1:0").await?;
    let addr = socket.local_addr().unwrap();
    let s = Arc::new(socket);
    let r = s.clone();

    let (tx, mut rx) = tokio::sync::mpsc::channel::<(Vec<u8>, std::net::SocketAddr)>(1_000);
    tokio::spawn_with_priority(async move {
        while let Some((bytes, addr)) = rx.recv().await {
            s.send_to(&bytes, &addr).await.unwrap();
        }
    }, 6);

    tokio::spawn_with_priority(async move {
        let mut buf = [0u8; 32];
        loop {
            let (len, addr) = r.recv_from(&mut buf).await.unwrap();
            tx.send((buf[..len].to_vec(), addr)).await.unwrap();
        }
    }, 5);

    // test that we can send a value and get back some response
    let sender = UdpSocket::bind("127.0.0.1:0").await?;
    sender.send_to(MSG, addr).await?;
    let mut recv_buf = [0u8; 32];
    let (len, _) = sender.recv_from(&mut recv_buf).await?;
    assert_eq!(&recv_buf[..len], MSG);
    Ok(())
}

#[tokio::test]
async fn split_chan_poll() -> std::io::Result<()> {
    // setup UdpSocket that will echo all sent items
    let socket = UdpSocket::bind("127.0.0.1:0").await?;
    let addr = socket.local_addr().unwrap();
    let s = Arc::new(socket);
    let r = s.clone();

    let (tx, mut rx) = tokio::sync::mpsc::channel::<(Vec<u8>, std::net::SocketAddr)>(1_000);
    tokio::spawn_with_priority(async move {
        while let Some((bytes, addr)) = rx.recv().await {
            poll_fn(|cx| s.poll_send_to(cx, &bytes, &addr))
                .await
                .unwrap();
        }
    }, 9);

    tokio::spawn_with_priority(async move {
        let mut recv_buf = [0u8; 32];
        let mut read = ReadBuf::new(&mut recv_buf);
        loop {
            let addr = poll_fn(|cx| r.poll_recv_from(cx, &mut read)).await.unwrap();
            tx.send((read.filled().to_vec(), addr)).await.unwrap();
        }
    }, 0);

    // test that we can send a value and get back some response
    let sender = UdpSocket::bind("127.0.0.1:0").await?;
    poll_fn(|cx| sender.poll_send_to(cx, MSG, &addr)).await?;

    let mut recv_buf = [0u8; 32];
    let mut read = ReadBuf::new(&mut recv_buf);
    let _ = poll_fn(|cx| sender.poll_recv_from(cx, &mut read)).await?;
    assert_eq!(read.filled(), MSG);
    Ok(())
}

// # Note
//
// This test is purposely written such that each time `sender` sends data on
// the socket, `receiver` awaits the data. On Unix, it would be okay waiting
// until the end of the test to receive all the data. On Windows, this would
// **not** be okay because it's resources are completion based (via IOCP).
// If data is sent and not yet received, attempting to send more data will
// result in `ErrorKind::WouldBlock` until the first operation completes.
#[tokio::test]
async fn try_send_spawn_with_priority() {
    const MSG2: &[u8] = b"world!";
    const MSG2_LEN: usize = MSG2.len();

    let sender = UdpSocket::bind("127.0.0.1:0").await.unwrap();
    let receiver = UdpSocket::bind("127.0.0.1:0").await.unwrap();

    receiver
        .connect(sender.local_addr().unwrap())
        .await
        .unwrap();

    let sent = &sender
        .try_send_to(MSG, receiver.local_addr().unwrap())
        .unwrap();
    assert_eq!(sent, &MSG_LEN);
    let mut buf = [0u8; 32];
    let mut received = receiver.recv(&mut buf[..]).await.unwrap();

    sender
        .connect(receiver.local_addr().unwrap())
        .await
        .unwrap();
    let sent = &sender.try_send(MSG2).unwrap();
    assert_eq!(sent, &MSG2_LEN);
    received += receiver.recv(&mut buf[..]).await.unwrap();

    std::thread::spawn_with_priority(move || {
        let sent = &sender.try_send(MSG).unwrap();
        assert_eq!(sent, &MSG_LEN);
    }, 8)
    .join()
    .unwrap();
    received += receiver.recv(&mut buf[..]).await.unwrap();

    assert_eq!(received, MSG_LEN * 2 + MSG2_LEN);
}