use std::{boxed::Box, collections::HashMap, pin::Pin, sync::Arc};

use anyhow::Result;
use futures::{Stream, StreamExt};
use tokio::{sync::mpsc, sync::Mutex};
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
}

impl RedisImpl {
    fn new() -> Self {
        Self {
            storage: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

#[tonic::async_trait]
impl Redis for RedisImpl {
    async fn set(&self, req: Request<Record>) -> Result<Response<Empty>, Status> {
        let req = req.into_inner();
        self.storage.lock().await.insert(req.key, req.value);
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

    type TxStream = ResponseStream<Record>;
    async fn tx(&self, req: Request<Streaming<TxOp>>) -> Result<Response<Self::TxStream>, Status> {
        let mut in_stream = req.into_inner();
        let (tx, rx) = mpsc::channel(4);

        let storage = self.storage.clone();
        tokio::spawn(async move {
            let mut storage = storage.lock().await;

            while let Some(op) = in_stream.next().await {
                match op {
                    Ok(TxOp { op: None }) => {}
                    Ok(TxOp {
                        op: Some(Op::Set(record)),
                    }) => {
                        storage.insert(record.key, record.value);
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
                    Err(_) => {
                        break;
                    }
                }
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
