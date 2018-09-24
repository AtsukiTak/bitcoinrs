use std::mem::drop;
use futures::{Async, Poll, Sink, Stream, future::{ok, poll_fn, Future}, sync::mpsc};
use tokio::executor::{DefaultExecutor, Executor, SpawnError};

pub trait Actor: Sized + Send + 'static
{
    type Message: Send;
    type ProcessFuture: Future<Item = Self, Error = ()> + Send;

    fn on_message(self, msg: Self::Message) -> Self::ProcessFuture;

    fn shutdown(self)
    {
        info!("Shutting down an Actor gracefully...");
    }
}

trait Handler<M: Message>: Actor
{
    type Response: Future<Item = Self, Error = ()> + Send;

    fn handle(self, msg: M) -> Self::Response;
}

#[derive(Debug, Clone)]
pub struct ActorRef<A: Actor>
{
    sender: mpsc::Sender<Mail<A::Message>>,
}

pub struct SendMsgFuture<A: Actor>
{
    sending: ::futures::sink::Send<mpsc::Sender<Mail<A::Message>>>,
}

pub struct GracefulShutdownFuture<A: Actor>
{
    sending: ::futures::sink::Send<mpsc::Sender<Mail<A::Message>>>,
}

#[derive(Debug)]
pub enum TrySendError<M>
{
    Disconnected(M),
    Full(M),
}

#[derive(Debug)]
pub struct SendError<M>
{
    pub msg: M,
}

pub struct ShutdownError();

pub struct FnActor<F>
{
    f: F,
}

pub fn spawn_actor<A>(actor: A) -> Result<ActorRef<A>, SpawnError>
where A: Actor
{
    let mut exe = DefaultExecutor::current();
    spawn_actor_with(actor, &mut exe)
}

pub fn spawn_actor_with<A, E>(actor: A, executor: &mut E) -> Result<ActorRef<A>, SpawnError>
where
    A: Actor,
    E: Executor,
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

    executor.spawn(Box::new(f))?;

    Ok(ActorRef::new(sender))
}

impl<A: Actor> ActorRef<A>
{
    fn new(sender: mpsc::Sender<Mail<A::Message>>) -> ActorRef<A>
    {
        ActorRef { sender }
    }

    pub fn try_send_msg(&mut self, msg: A::Message) -> Result<(), TrySendError<A::Message>>
    {
        self.sender
            .try_send(Mail::Msg(msg))
            .map_err(TrySendError::from_try_send_mail_err)
    }

    pub fn graceful_shutdown(self) -> GracefulShutdownFuture<A>
    {
        GracefulShutdownFuture {
            sending: self.sender.send(Mail::GracefulShutdown),
        }
    }
}

/*
impl<A: Actor> Sink for ActorRef<A>
{
    type SinkItem = A::Message;
    type SinkError = SendError<A>;

    fn start_send(&mut self, item: Self::SinkItem) -> StartSend<Self::SinkItem, Self::SinkError>
    {
        match self.sender.start_send(item) {
            Ok(AsyncSink::NotReady(item)) => Ok(AsyncSink::NotReady(item)),
            Ok(AsyncSink::Ready) => Ok(AsyncSink::Ready),
            Err(e) => Err(SendError::from_send_mail_err(e)),
        }
    }

    fn poll_complete(&mut self) -> Poll<(), Self::SinkError>
    {
        match self.sender.poll_complete() {
            Ok(Async::Ready(())) => Ok(Async::Ready(())),
            Ok(Async::NotReady) => Ok(Async::NotReady),
            Err(e) => Err(SendError::from_send_mail_err(e)),
        }
    }
}
*/

impl<A: Actor> Future for GracefulShutdownFuture<A>
{
    type Item = ();
    type Error = ShutdownError;
    fn poll(&mut self) -> Poll<Self::Item, Self::Error>
    {
        match self.sending.poll() {
            Ok(Async::NotReady) => Ok(Async::NotReady),
            Ok(Async::Ready(sender)) => {
                drop(sender);
                Ok(Async::Ready(()))
            },
            Err(e) => Err(ShutdownError::from_send_mail_err(e)),
        }
    }
}

impl<M> TrySendError<M>
{
    /// Converts `futures::sync::mpsc::TrySendError<Mail<M>>` into `TrySendError<M>`.
    ///
    /// # Panic
    /// if `Mail` wrapped by `e` is `Mail::GracefulShutdown`.
    fn from_try_send_mail_err(e: mpsc::TrySendError<Mail<M>>) -> TrySendError<M>
    {
        if e.is_full() {
            TrySendError::Full(e.into_inner().into_msg().unwrap())
        } else {
            TrySendError::Disconnected(e.into_inner().into_msg().unwrap())
        }
    }
}

impl<M> SendError<M>
{
    /// Converts `futures::sync::mpsc::SendError<Mail<M>>` into `SendError<M>`.
    ///
    /// # Panic
    /// if `Mail` wrapped by `e` is `Mail::GracefulShutdown`.
    fn from_send_mail_err(e: mpsc::SendError<Mail<M>>) -> SendError<M>
    {
        let mail = e.into_inner();
        if !mail.is_msg() {
            panic!("Given error does not represents fail to send Msg");
        }
        SendError {
            msg: mail.into_msg().unwrap(),
        }
    }
}

impl ShutdownError
{
    /// Converts `futures::sync::mpsc::SendError<Mail<M>>` into `ShutdownError`.
    ///
    /// # Panic
    /// if `Mail` wrapped by `e` is `Mail::Msg<M>`.
    fn from_send_mail_err<M>(e: mpsc::SendError<Mail<M>>) -> ShutdownError
    {
        let mail = e.into_inner();
        if !mail.is_graceful_shutdown() {
            panic!("Given error does not represents fail to send GracefulShutdown.");
        }
        ShutdownError()
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

    fn is_msg(&self) -> bool
    {
        match self {
            &Mail::Msg(_) => true,
            _ => false,
        }
    }

    fn is_graceful_shutdown(&self) -> bool
    {
        match self {
            &Mail::GracefulShutdown => true,
            _ => false,
        }
    }
}
