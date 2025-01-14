use error::Result;
use std::marker;
use std::thread;

pub enum Command {
    Continue,
    Quit,
}

pub trait Receiver<T> {
    // Recvs messages from mpsc, tcp, udp or other protocols
    fn recv(&mut self) -> Result<T>;
}

pub trait Handler<T> {
    fn handle(&mut self, t: T) -> Result<Command>;
}

pub struct Spinner<T, R, H>
where
    R: Receiver<T>,
    H: Handler<T>,
{
    recver: R,
    handler: H,
    phantom: marker::PhantomData<T>,
}

impl<T, R, H> Spinner<T, R, H>
where
    R: Receiver<T>,
    H: Handler<T>,
{
    pub fn new(recver: R, handler: H) -> Spinner<T, R, H> {
        Spinner {
            recver: recver,
            handler: handler,
            phantom: marker::PhantomData {},
        }
    }

    pub fn spin(&mut self) -> Result<()> {
        loop {
            let command = self.recver.recv()?;
            match self.handler.handle(command)? {
                Command::Quit => break,
                Command::Continue => (),
            };
        }
        Ok(())
    }
}

pub fn spawn<T, R, H>(recver: R, handler: H) -> thread::JoinHandle<()>
where
    T: 'static,
    R: 'static + Send + Receiver<T>,
    H: 'static + Send + Handler<T>,
{
    thread::spawn(move || {
        if let Err(error) = Spinner::new(recver, handler).spin() {
            println!("#sirver spin_forever: {:#?}", error);
        }
    })
}
