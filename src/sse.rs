use actix_web::web::{Bytes, Data};
use actix_web::Error;

use futures::stream::Stream;
use tokio::sync::mpsc::{channel, Receiver, Sender};
use tokio::time::{interval, Duration};

use std::pin::Pin;
use std::sync::Mutex;

use crate::db::polls_repo::PollResponse;
use crate::models::poll_api_model::PollResults;

pub struct Broadcaster {
    clients: Vec<Sender<Bytes>>,
}

impl Broadcaster {
    pub fn create() -> Data<Mutex<Self>> {
        let me = Data::new(Mutex::new(Broadcaster::new()));
        me
    }

    pub fn new() -> Self {
        Broadcaster {
            clients: Vec::new(),
        }
    }

    pub async fn spawn_ping(me: Data<Mutex<Self>>) {
        let mut interval = interval(Duration::from_secs(5));

        loop {
            interval.tick().await;

            let mut broadcaster = me.lock().unwrap();
            broadcaster.remove_stale_clients();
        }
    }

    pub fn remove_stale_clients(&mut self) {
        self.clients.retain(|client| {
            client
                .clone()
                .try_send(Bytes::from("data: ping\n\n"))
                .is_ok()
        });
    }

    pub fn new_client(&mut self) -> Client {
        let (tx, rx) = channel(100);

        // Send initial connection message
        let _ = tx.clone().try_send(Bytes::from("data: connected\n\n"));

        self.clients.push(tx);
        Client(rx)
    }

    pub fn send(&self, msg: &str) {
        let msg = Bytes::from(format!("data: {}\n\n", msg));

        for client in &self.clients {
            let _ = client.clone().try_send(msg.clone());
        }
    }

    pub fn send_updated_poll(&self, poll: &PollResponse) {
        let poll_json = serde_json::to_string(poll).unwrap();

        let msg = Bytes::from(format!("event: poll_updated\ndata: {}\n\n", poll_json));

        for client in &self.clients {
            let _ = client.clone().try_send(msg.clone());
        }
    }

    pub fn send_poll_results(&self, response: &PollResults) {
        let poll_result_json = format!("{:?}", serde_json::to_string(response).unwrap());

        let msg = Bytes::from(format!(
            "event: poll_results\ndata: {}\n\n",
            poll_result_json
        ));

        for client in &self.clients {
            let _ = client.clone().try_send(msg.clone());
        }
    }
}

// Wrap Receiver in own type with correct error handling
pub struct Client(Receiver<Bytes>);

impl Stream for Client {
    type Item = Result<Bytes, Error>;

    fn poll_next(
        mut self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        match Pin::new(&mut self.0).poll_recv(cx) {
            std::task::Poll::Ready(Some(item)) => std::task::Poll::Ready(Some(Ok(item))),
            std::task::Poll::Ready(None) => std::task::Poll::Ready(None),
            std::task::Poll::Pending => std::task::Poll::Pending,
        }
    }
}
