#[derive(Debug, Fail)]
pub enum ConnectionError
{
    #[fail(display = "Detect misbehavior peer")]
    MisbehavePeer,
}
