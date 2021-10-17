use actix::{Actor, Context, ContextFutureSpawner, Handler, WrapFuture};
use ferrum_shared::broker::BrokerEvent;
use lapin::{options::BasicPublishOptions, BasicProperties, Channel};
use serde::{Deserialize, Serialize};

pub struct Broker {
    pub queue: String,
    pub channel: Channel,
}

impl Actor for Broker {
    type Context = Context<Self>;
}

#[derive(Debug, Clone, Serialize, Deserialize, actix::prelude::Message)]
#[rtype(result = "()")]
pub struct PublishBrokerEvent {
    pub broker_event: BrokerEvent,
}

impl Handler<PublishBrokerEvent> for Broker {
    type Result = ();

    fn handle(&mut self, msg: PublishBrokerEvent, ctx: &mut Self::Context) -> Self::Result {
        let serialized_message = serde_json::to_vec(&msg.broker_event).unwrap();

        let channel = self.channel.clone();
        let queue = self.queue.clone();

        async move {
            channel
                .basic_publish(
                    "",
                    &queue,
                    BasicPublishOptions::default(),
                    serialized_message,
                    BasicProperties::default(),
                )
                .await
                .unwrap()
                .await
                .unwrap();
        }
        .into_actor(self)
        .wait(ctx)
    }
}
