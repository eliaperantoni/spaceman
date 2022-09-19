use std::{boxed::Box, pin::Pin, time::Duration};

use anyhow::Result;
use futures::{Stream, StreamExt};
use sha2::{Digest, Sha256};
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tonic::{transport::Server, Request, Response, Status, Streaming};

use pb::{
    math_request::Op,
    playground_server::{Playground, PlaygroundServer},
    CountdownRequest, CountdownResponse, HangmanRequest, HangmanResponse, HashRequest,
    HashResponse, MathRequest, MathResponse, SecretRequest, SecretResponse,
};

mod pb {
    tonic::include_proto!("playground");
}

type ResponseStream<T> = Pin<Box<dyn Stream<Item = Result<T, Status>> + Send>>;

#[derive(Default)]
struct PlaygroundImpl;

#[tonic::async_trait]
impl Playground for PlaygroundImpl {
    async fn math(&self, req: Request<MathRequest>) -> Result<Response<MathResponse>, Status> {
        let (lhs, rhs) = (req.get_ref().lhs, req.get_ref().rhs);
        let result = match req.get_ref().op() {
            Op::Add => lhs + rhs,
            Op::Subtract => lhs - rhs,
            Op::Multiply => lhs * rhs,
            Op::Divide => lhs / rhs,
        };
        Ok(Response::new(MathResponse { result }))
    }

    type CountdownStream = ResponseStream<CountdownResponse>;
    async fn countdown(
        &self,
        req: Request<CountdownRequest>,
    ) -> Result<Response<Self::CountdownStream>, Status> {
        let mut left = req.get_ref().seconds;
        let (tx, rx) = mpsc::channel(4);

        tokio::spawn(async move {
            loop {
                if tx.send(Ok(CountdownResponse { left })).await.is_err() {
                    break;
                }

                if left == 0 {
                    break;
                }

                left -= 1;
                tokio::time::sleep(Duration::from_secs(1)).await;
            }
        });

        Ok(Response::new(Box::pin(ReceiverStream::new(rx))))
    }

    async fn hash(
        &self,
        mut req: Request<Streaming<HashRequest>>,
    ) -> Result<Response<HashResponse>, Status> {
        let mut hasher = Sha256::new();
        while let Some(piece) = req.get_mut().next().await {
            if let Ok(piece) = piece {
                hasher.update(&piece.piece);
            } else {
                break;
            }
        }

        Ok(Response::new(HashResponse {
            hash: hasher.finalize().to_vec(),
        }))
    }

    type HangmanStream = ResponseStream<HangmanResponse>;
    async fn hangman(
        &self,
        _req: Request<Streaming<HangmanRequest>>,
    ) -> Result<Response<Self::HangmanStream>, Status> {
        todo!()
    }

    async fn secret(
        &self,
        _req: Request<SecretRequest>,
    ) -> Result<Response<SecretResponse>, Status> {
        todo!()
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let addr = "0.0.0.0:7575".parse()?;
    let pg = PlaygroundImpl::default();

    println!("Listening on {}", &addr);

    Server::builder()
        .add_service(PlaygroundServer::new(pg))
        .serve(addr)
        .await?;
    Ok(())
}
