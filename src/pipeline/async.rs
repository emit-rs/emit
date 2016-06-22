use collectors;
use events::Event;
use std::sync::mpsc::{Sender, Receiver};
use std::thread;
use std::mem;
use std::sync::Mutex;
use pipeline::chain::Emit;

pub enum Item {
    Done,
    Emit(Event<'static>)
}

pub struct AsyncCollector{
    chan: Sender<Item>,
    worker: Option<thread::JoinHandle<()>>
}

impl Drop for AsyncCollector{
    fn drop(&mut self) {
        self.chan.send(Item::Done).is_ok();
        let thread = mem::replace(&mut self.worker, None);
        if let Some(handle) = thread {
            handle.join().is_ok();
        }
    }
}

impl AsyncCollector{
    pub fn new<T: collectors::AcceptEvents + Send + 'static>(collector: T, tx: Sender<Item>, rx: Receiver<Item>) -> AsyncCollector{
        let coll = collector;
        let child = thread::spawn(move|| {
            loop {
                let done = match rx.recv().unwrap() {
                    Item::Done => true,
                    Item::Emit(payload) => {
                        if let Err(e) = coll.accept_events(&vec![payload]) {
                            error!("Could not dispatch events: {}", e);
                        }
                        false
                    }
                };
                
                if done {
                    break;
                }
            }
        });
           
        AsyncCollector{
            worker: Some(child),
            chan: tx
        }
    }
}

pub struct SenderElement {
    chan: Mutex<Sender<Item>>
}

impl SenderElement {
    pub fn new(chan: Sender<Item>) -> SenderElement {
         SenderElement {chan: Mutex::new(chan)}
    }
}

impl Emit for SenderElement {
    fn emit(&self, event: Event<'static>) {
        self.chan.lock().unwrap().send(Item::Emit(event)).expect("The event could not be emitted to the pipeline");
    }
}
