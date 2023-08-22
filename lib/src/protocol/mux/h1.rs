use sozu_command::ready::Ready;

use crate::{
    protocol::mux::{Context, GlobalStreamId, Position},
    socket::{SocketHandler, SocketResult},
    Readiness,
};

pub struct ConnectionH1<Front: SocketHandler> {
    pub position: Position,
    pub readiness: Readiness,
    pub socket: Front,
    /// note: a Server H1 will always reference stream 0, but a client can reference any stream
    pub stream: GlobalStreamId,
}

impl<Front: SocketHandler> ConnectionH1<Front> {
    pub fn readable(&mut self, context: &mut Context) {
        println!("======= MUX H1 READABLE");
        let stream = &mut context.streams.get(self.stream);
        let kawa = match self.position {
            Position::Client => &mut stream.front,
            Position::Server => &mut stream.back,
        };
        let (size, status) = self.socket.socket_read(kawa.storage.space());
        println!("  size: {size}, status: {status:?}");
        if size > 0 {
            kawa.storage.fill(size);
        } else {
            self.readiness.event.remove(Ready::READABLE);
        }
        match status {
            SocketResult::Continue => {}
            SocketResult::Closed => todo!(),
            SocketResult::Error => todo!(),
            SocketResult::WouldBlock => self.readiness.event.remove(Ready::READABLE),
        }
        kawa::h1::parse(kawa, &mut kawa::h1::NoCallbacks);
        kawa::debug_kawa(kawa);
        if kawa.is_terminated() {
            self.readiness.interest.remove(Ready::READABLE);
        }
    }
    pub fn writable(&mut self, context: &mut Context) {
        println!("======= MUX H1 WRITABLE");
        let stream = &mut context.streams.get(self.stream);
        let kawa = match self.position {
            Position::Client => &mut stream.back,
            Position::Server => &mut stream.front,
        };
        kawa.prepare(&mut kawa::h1::BlockConverter);
        let bufs = kawa.as_io_slice();
        if bufs.is_empty() {
            self.readiness.interest.remove(Ready::WRITABLE);
            return;
        }
        let (size, status) = self.socket.socket_write_vectored(&bufs);
        println!("  size: {size}, status: {status:?}");
        if size > 0 {
            kawa.consume(size);
            // self.backend_readiness.interest.insert(Ready::READABLE);
        } else {
            self.readiness.event.remove(Ready::WRITABLE);
        }
    }
}
