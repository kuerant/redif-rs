
use std::collections::HashMap;
use std::io::Result;
use std::net::{TcpListener, TcpStream, SocketAddr};

use amy::{Notification, Event, Poller, Registrar};
use frame_reader::FrameReader;
use frame_writer::FrameWriter;

use Handler;
use std::sync::{Arc,Mutex};

/// Redif framework entry point
///
/// redif should be invoke with a TCP port and a request handler 
/// where user customize action taken on data.
///
pub fn run<T: Send + Handler + 'static>(port: u16, handler: Arc<Mutex<T>>) -> Result<()> {
    use std::thread;
    use std::sync::mpsc::channel;

    info!("Listening on port {} ...", port);

    let addr = format!("0.0.0.0:{}", port);

    let mut poller = Poller::new().unwrap();
    let registrar = poller.get_registrar().unwrap();

    let (tx, rx) = channel();

    let handle = thread::spawn(move || {
        let listener = TcpListener::bind(&addr).unwrap();
        listener.set_nonblocking(true).unwrap();

        let listener_id = registrar.register(&listener, Event::Read).unwrap();

        let mut connections = HashMap::new();

        loop {
            let notification : Notification = rx.recv().unwrap();
            if notification.id == listener_id {
               //let (mut socket, _) = listener.accept().unwrap();
               let (socket, address) = listener.accept().unwrap();
               socket.set_nonblocking(true).unwrap();

               let socket_id = registrar.register(&socket, Event::Both).unwrap();
               info!("DEBUG accept socket#{} {:?} {:?} ...", socket_id, &socket, &address);

               let conn = Conn {
                   sock: socket,
                   addr: address,
                   reader: FrameReader::new(1024 * 1024),
                   writer: FrameWriter::new(),
               };
               connections.insert(socket_id, conn);
            } else {
                if let Err(e) = handle_poll_notification(&notification, &registrar, &mut connections, handler.clone()) {
                    if let Some(conn) = connections.remove(&notification.id) {
                        registrar.deregister(&conn.sock).unwrap();
                        error!("fail to handle poll notification Event::{:?} sock#{} {} -- {}", &notification.event, &notification.id, &conn.addr, e);
                    } else {
                        error!("fail to handle poll notification Event::{:?} sock#{} -- {}", &notification.event, &notification.id, e);
                    }
                }
            }
        }
    });

    let handle_poller = thread::Builder::new().name(format!("poller")).spawn(move || {
        loop {
            let notifications = poller.wait(5000).unwrap();
            for n in notifications {
                tx.send(n).unwrap();
            }
        }
    }).unwrap();

    handle.join().unwrap();
    handle_poller.join().unwrap();

    Ok(())
}



struct Conn {
    sock: TcpStream,
    addr: SocketAddr,
    reader: FrameReader,
    writer: FrameWriter
}

// Assume only TcpStream notifications for now. Error handling is done by the (elided) caller.
fn handle_poll_notification<T: Send + Handler>(notification: &Notification,
                            _registrar: &Registrar,
                            connections: &mut HashMap<usize, Conn>,
                            handler: Arc<Mutex<T>>) -> Result<()> {
    //info!("DEBUG handle notification {:?} ...", notification);
    use value::Value;

    if let Some(conn) = connections.get_mut(&notification.id) {
        match notification.event {
            Event::Read => {
                // Try to read the data from the connection sock. Ignore the amount of bytes read.
                let _sz = conn.reader.read(&mut conn.sock)?;

                // Iterate through all available complete messages. Note that this iterator is mutable
                // in a non-traditional sense. It returns each complete message only once and removes it
                // from the reader.
                for msg in conn.reader.iter_mut() {
                    //println!("Received a complete message: {:?}", ::std::str::from_utf8(&msg).unwrap());
                    println!("Received a complete message: {:?}", &msg);

                    let data = Value::Status("OK".to_string()).encode();
                    let _sz = conn.writer.write(&mut conn.sock, Some(data))?;
                    //info!("DEBUG write {}", _sz);
                }
            },
            Event::Write => {
                // Attempt to write *all* existing data queued for writing. `None` as the second
                // parameter means no new data.
                conn.writer.write(&mut conn.sock, None)?;
            },
            Event::Both => {
                //info!("DEBUG socket#{} Event::Both read ...", notification.id);
                let _sz = conn.reader.read(&mut conn.sock)?;
                //info!("DEBUG socket#{} Event::Both read {} bytes", notification.id, _sz);

                for msg in conn.reader.iter_mut() {
                    //println!("Received a complete message: {:?}", &msg);
                    let mut handler = handler.lock().unwrap();
                    if let Some(data) = handler.handle( &msg ) {
                        conn.writer.write(&mut conn.sock, Some(data.encode()))?;
                    }
                }

                let _sz = conn.writer.write(&mut conn.sock, None)?;
            }
        }
    } else {
        error!("SKIP notification for un-registered socket#{}", notification.id);
    }

    Ok(())
}


