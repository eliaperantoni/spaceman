use std::{
    boxed::Box,
    collections::{HashMap, HashSet},
    pin::Pin,
    sync::Arc,
};

use anyhow::Result;
use futures::{Stream, StreamExt};
use tokio::{sync::broadcast, sync::mpsc, sync::Mutex};
use tokio_stream::wrappers::ReceiverStream;
use tonic::{transport::Server, Request, Response, Status, Streaming};

use pb::{
    redis_server::{Redis, RedisServer},
    tx_op::Op,
    Empty, Key, Record, TxOp,
};

mod pb {
    tonic::include_proto!("redis");
}

type ResponseStream<T> = Pin<Box<dyn Stream<Item = Result<T, Status>> + Send>>;

struct RedisImpl {
    storage: Arc<Mutex<HashMap<String, String>>>,
    broadcast: broadcast::Sender<String>,
}

impl RedisImpl {
    fn new() -> Self {
        Self {
            storage: Arc::new(Mutex::new(HashMap::new())),
            broadcast: broadcast::channel(16).0,
        }
    }
}

#[tonic::async_trait]
impl Redis for RedisImpl {
    async fn set(&self, req: Request<Record>) -> Result<Response<Empty>, Status> {
        let req = req.into_inner();
        self.storage.lock().await.insert(req.key.clone(), req.value);
        self.broadcast
            .send(req.key)
            .map_err(|_| Status::internal("broadcasting change to watchers"))?;
        Ok(Response::new(Empty {}))
    }

    async fn get(&self, req: Request<Key>) -> Result<Response<Record>, Status> {
        let req = req.into_inner();
        if let Some(value) = self.storage.lock().await.get(&req.key) {
            Ok(Response::new(Record {
                key: req.key,
                value: value.clone(),
            }))
        } else {
            Err(Status::not_found("key not found"))
        }
    }

    type WatchStream = ResponseStream<Record>;
    async fn watch(&self, req: Request<Key>) -> Result<Response<Self::WatchStream>, Status> {
        let key = req.into_inner().key;

        let (tx, rx) = mpsc::channel(16);

        let mut broadcast = self.broadcast.subscribe();
        let storage = self.storage.clone();
        tokio::spawn(async move {
            while let Ok(changed_key) = broadcast.recv().await {
                if changed_key != key {
                    continue;
                }

                let storage = storage.lock().await;
                let new_value = storage.get(&changed_key).unwrap();
                match tx
                    .send(Ok(Record {
                        key: changed_key,
                        value: new_value.clone(),
                    }))
                    .await
                {
                    Ok(_) => {}
                    Err(_err) => {
                        break;
                    }
                }
            }
        });

        let out_stream = ReceiverStream::new(rx);
        Ok(Response::new(Box::pin(out_stream)))
    }

    type TxStream = ResponseStream<Record>;
    async fn tx(&self, req: Request<Streaming<TxOp>>) -> Result<Response<Self::TxStream>, Status> {
        let mut in_stream = req.into_inner();
        let (tx, rx) = mpsc::channel(16);

        let storage = self.storage.clone();
        let broadcast = self.broadcast.clone();
        tokio::spawn(async move {
            let mut storage = storage.lock().await;
            let mut changed_keys = HashSet::new();

            while let Some(op) = in_stream.next().await {
                match op {
                    Ok(TxOp { op: None }) => {}
                    Ok(TxOp {
                        op: Some(Op::Set(record)),
                    }) => {
                        storage.insert(record.key.clone(), record.value);
                        changed_keys.insert(record.key);
                    }
                    Ok(TxOp {
                        op: Some(Op::Get(key)),
                    }) => {
                        if let Some(value) = storage.get(&key.key) {
                            tx.send(Ok(Record {
                                key: key.key,
                                value: value.clone(),
                            }))
                            .await
                            .expect("working tx");
                        } else {
                            tx.send(Err(Status::not_found("key not found")))
                                .await
                                .expect("working tx");
                        }
                    }
                    Err(_err) => {
                        break;
                    }
                }
            }

            for changed_key in changed_keys.into_iter() {
                broadcast.send(changed_key).expect("working broadcast");
            }
        });

        let out_stream = ReceiverStream::new(rx);
        Ok(Response::new(Box::pin(out_stream)))
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let addr = "0.0.0.0:7575".parse()?;
    let redis = RedisImpl::new();

    println!("Listening on {}", &addr);

    Server::builder()
        .add_service(RedisServer::new(redis))
        .serve(addr)
        .await?;
    Ok(())
}
