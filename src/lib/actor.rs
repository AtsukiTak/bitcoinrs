use std::marker::PhantomData;
use futures::{Async, Stream, future::{ok, poll_fn, Future}, sync::mpsc};
use tokio::spawn;

pub trait Actor: Sized + Sync + Send
{
    type Message: Sync + Send;
    type ProcessFuture: Future<Item = Self, Error = ()> + Sync + Send;

    fn on_message(self, msg: Self::Message) -> Self::ProcessFuture;

    fn shutdown(self)
    {
        info!("Shutdown an Actor gracefully");
    }
}

#[derive(Debug, Clone)]
pub struct ActorRef<A: Actor>
{
    _actor: PhantomData<A>,
    sender: mpsc::Sender<Mail<A::Message>>,
}

#[derive(Debug)]
pub enum SendError<M>
{
    Disconnected(M),
    Full(M),
}

pub fn spawn_actor<A: Actor + 'static>(actor: A) -> ActorRef<A>
{
    let (sender, receiver) = mpsc::channel(42);

    let f = receiver
        .take_while(|mail| {
            match mail {
                &Mail::Msg(_) => ok(true),
                &Mail::GracefulShutdown => ok(false),
            }
        })
        .fold(Some(actor), |should_actor, mail| {
            let actor = should_actor.unwrap();
            let mut maybe_fut = match mail {
                Mail::Msg(m) => Some(actor.on_message(m).map(|actor| Some(actor))),
                Mail::GracefulShutdown => {
                    actor.shutdown();
                    None
                },
            };
            poll_fn(move || {
                match &mut maybe_fut {
                    &mut Some(ref mut fut) => fut.poll(),
                    &mut None => Ok(Async::Ready(None)),
                }
            })
        })
        .map(|_should_none| ());

    spawn(f);

    ActorRef::new(sender)
}

impl<A: Actor> ActorRef<A>
{
    fn new(sender: mpsc::Sender<Mail<A::Message>>) -> ActorRef<A>
    {
        ActorRef {
            _actor: PhantomData,
            sender,
        }
    }

    pub fn send_msg(&mut self, msg: A::Message) -> Result<(), SendError<A::Message>>
    {
        self.sender
            .try_send(Mail::Msg(msg))
            .map_err(|e| SendError::fail_send_msg(e))
    }

    pub fn graceful_shutdown(mut self) -> Result<(), SendError<()>>
    {
        self.sender
            .try_send(Mail::GracefulShutdown)
            .map_err(|e| SendError::fail_graceful_shutdown(e))
    }
}

impl<M> SendError<M>
{
    fn fail_send_msg(e: mpsc::TrySendError<Mail<M>>) -> SendError<M>
    {
        if e.is_full() {
            SendError::Full(e.into_inner().into_msg().unwrap())
        } else {
            // disconnected
            SendError::Disconnected(e.into_inner().into_msg().unwrap())
        }
    }
}

impl SendError<()>
{
    fn fail_graceful_shutdown<M>(e: mpsc::TrySendError<Mail<M>>) -> SendError<()>
    {
        if e.is_full() {
            SendError::Full(())
        } else {
            // disconnected
            SendError::Disconnected(())
        }
    }
}

#[derive(Debug)]
enum Mail<M>
{
    Msg(M),
    GracefulShutdown,
}

impl<M> Mail<M>
{
    fn into_msg(self) -> Option<M>
    {
        match self {
            Mail::Msg(m) => Some(m),
            Mail::GracefulShutdown => None,
        }
    }
}
