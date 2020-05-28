use std::collections::HashMap as StdHash;

use actix::dev::{MessageResponse, ResponseChannel};
use actix::prelude::*;

use im::HashMap;

use serde::{Deserialize, Serialize};

use failure::Error;

use futures::FutureExt;

use uuid::Uuid;

use tokio::net::UnixStream;
use tokio::sync::oneshot;

use cliff::client::{Delegate, InterfaceRequest, WriteInterface};
use cliff::server::{IpcServer, Router};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: String,
    pub name: String,
    pub complete: bool,
}

impl Task {
    pub fn new(name: String) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name,
            complete: false,
        }
    }
}

#[derive(Debug, Default)]
pub struct TaskStore {
    tasks: HashMap<String, Task>,
}

impl Actor for TaskStore {
    type Context = Context<Self>;
}

impl Handler<TaskCommandRequest> for TaskStore {
    type Result = TaskCommandResponse;

    fn handle(&mut self, msg: TaskCommandRequest, _ctx: &mut Self::Context) -> Self::Result {
        use TaskCommandRequest::*;

        match msg {
            Create { id, name } => {
                println!("Inserting task {}: {}", id, name);
                let task = Task::new(name);
                self.tasks.insert(task.id.clone(), task);
                TaskCommandResponse::Success { id }
            }
            Complete { id, task_id } => match self.tasks.get_mut(&task_id) {
                Some(task) if !task.complete => {
                    println!("Completing task {}: {}", id, task.name);
                    task.complete = true;
                    TaskCommandResponse::Success { id }
                }
                Some(_) => TaskCommandResponse::Error {
                    id,
                    description: "Task Already Complete".to_string(),
                },
                None => TaskCommandResponse::Error {
                    id,
                    description: "Task Not Found".to_string(),
                },
            },
        }
    }
}

impl Router<TaskCommandRequest> for TaskStore {}

impl Handler<TaskQueryRequest> for TaskStore {
    type Result = TaskQueryResponse;

    fn handle(&mut self, msg: TaskQueryRequest, _ctx: &mut Self::Context) -> Self::Result {
        use TaskQueryRequest::*;
        use TaskQueryResponse::*;

        match msg {
            GetOne { id, query } => match self.tasks.values().find(|task| query.eval(task)) {
                Some(task) => TaskQueryResponse::Task {
                    id,
                    task: task.clone(),
                },
                None => NotFound { id },
            },
            Get { id, query } => {
                let tasks = self
                    .tasks
                    .values()
                    .filter(|task| query.eval(task))
                    .cloned()
                    .collect();

                Tasks { id, tasks }
            }
        }
    }
}

impl Router<TaskQueryRequest> for TaskStore {}

pub fn serve_command(path: &str, addr: &Addr<TaskStore>) -> Result<(), Error> {
    IpcServer::<TaskCommandRequest, TaskStore>::serve(path, addr.clone())
}

pub fn serve_query(path: &str, addr: &Addr<TaskStore>) -> Result<(), Error> {
    IpcServer::<TaskQueryRequest, TaskStore>::serve(path, addr.clone())
}

// Task Commands
#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "message", content = "data")]
pub enum TaskCommandRequest {
    Create { id: String, name: String },
    Complete { id: String, task_id: String },
}

impl Message for TaskCommandRequest {
    type Result = TaskCommandResponse;
}

#[derive(Serialize, Deserialize, Message, Debug)]
#[rtype(result = "()")]
#[serde(tag = "message", content = "data")]
pub enum TaskCommandResponse {
    Success { id: String },
    Error { id: String, description: String },
}

impl<A, M> MessageResponse<A, M> for TaskCommandResponse
where
    A: Actor,
    M: Message<Result = TaskCommandResponse>,
{
    fn handle<R: ResponseChannel<M>>(self, _: &mut A::Context, tx: Option<R>) {
        if let Some(tx) = tx {
            tx.send(self);
        }
    }
}

// Task Queries
#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "type", content = "params")]
pub enum NameQuery {
    Contains(String),
    Is(String),
}

#[derive(Serialize, Deserialize, Debug)]
pub enum QueryOp {
    And,
    Or,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "type", content = "params")]
pub enum TaskQuery {
    Id(String),
    Done(bool),
    Name(NameQuery),
    Compound(QueryOp, Vec<TaskQuery>),
}

impl TaskQuery {
    fn eval(&self, task: &Task) -> bool {
        use NameQuery::*;
        use QueryOp::*;
        use TaskQuery::*;

        match self {
            Id(id) => *id == task.id,
            Done(is_done) => *is_done == task.complete,
            Name(Contains(partial)) => task.name.contains(partial),
            Name(Is(total)) => *total == task.name,
            Compound(And, queries) => queries.iter().all(|sub_query| sub_query.eval(task)),
            Compound(Or, queries) => queries.iter().any(|sub_query| sub_query.eval(task)),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "message", content = "data")]
pub enum TaskQueryRequest {
    GetOne { id: String, query: TaskQuery },
    Get { id: String, query: TaskQuery },
}

impl Message for TaskQueryRequest {
    type Result = TaskQueryResponse;
}

#[derive(Serialize, Message, Deserialize, Debug)]
#[rtype(result = "()")]
#[serde(tag = "message", content = "data")]
pub enum TaskQueryResponse {
    NotFound { id: String },
    Task { id: String, task: Task },
    Tasks { id: String, tasks: Vec<Task> },
    // TODO: Add Query Subscription
}

impl<A, M> MessageResponse<A, M> for TaskQueryResponse
where
    A: Actor,
    M: Message<Result = TaskQueryResponse>,
{
    fn handle<R: ResponseChannel<M>>(self, _: &mut A::Context, tx: Option<R>) {
        if let Some(tx) = tx {
            tx.send(self);
        }
    }
}

#[derive(Debug, Message)]
#[rtype(result = "()")]
pub enum TaskCommand {
    Create { name: String },
    Complete { task_id: String },
}

pub struct CommandClient {
    writer: Addr<WriteInterface<TaskCommandRequest>>,
    futures: StdHash<String, oneshot::Sender<()>>,
}

impl Actor for CommandClient {
    type Context = Context<Self>;
}

impl Handler<TaskCommand> for CommandClient {
    type Result = ResponseFuture<()>;

    fn handle(&mut self, msg: TaskCommand, _ctx: &mut Self::Context) -> Self::Result {
        let id = Uuid::new_v4().to_string();
        let (tx, rx) = oneshot::channel();

        self.futures.insert(id.clone(), tx);

        let req = match msg {
            TaskCommand::Create { name } => {
                InterfaceRequest(TaskCommandRequest::Create { id, name })
            }
            TaskCommand::Complete { task_id } => {
                InterfaceRequest(TaskCommandRequest::Complete { id, task_id })
            }
        };

        Box::pin(self.writer.send(req).then(|_res| async move {
            rx.await.unwrap();
        }))
    }
}

impl StreamHandler<Result<TaskCommandResponse, Error>> for CommandClient {
    fn handle(&mut self, item: Result<TaskCommandResponse, Error>, _ctx: &mut Self::Context) {
        match item {
            Ok(TaskCommandResponse::Success { id }) => {
                if let Some(tx) = self.futures.remove(&id) {
                    tx.send(()).unwrap();
                }
            }
            Ok(TaskCommandResponse::Error { id, description }) => {
                if let Some(tx) = self.futures.remove(&id) {
                    println!("Remote Error: {}", description);
                    tx.send(()).unwrap();
                }
            }
            Err(e) => {
                println!("Error: {:?}", e);
            }
        }
    }
}

impl CommandClient {
    #[allow(dead_code)]
    pub async fn connect(path: &str) -> Result<Addr<Self>, Error> {
        let stream = UnixStream::connect(path).await?;
        let (r, w) = tokio::io::split(stream);

        let writer = WriteInterface::attach(w).await?;

        let addr = CommandClient::create(|ctx| {
            CommandClient::listen(r, ctx);

            CommandClient {
                writer,
                futures: StdHash::new(),
            }
        });

        Ok(addr)
    }
}

#[derive(Debug)]
pub struct GetQuery(pub TaskQuery);

impl Message for GetQuery {
    type Result = Vec<Task>;
}

#[derive(Debug)]
pub struct GetOneQuery(pub TaskQuery);

impl Message for GetOneQuery {
    type Result = Option<Task>;
}

pub struct QueryClient {
    writer: Addr<WriteInterface<TaskQueryRequest>>,
    get_futures: StdHash<String, oneshot::Sender<Vec<Task>>>,
    get_one_futures: StdHash<String, oneshot::Sender<Option<Task>>>,
}

impl Actor for QueryClient {
    type Context = Context<Self>;
}

impl Handler<GetQuery> for QueryClient {
    type Result = ResponseFuture<Vec<Task>>;

    fn handle(&mut self, msg: GetQuery, _ctx: &mut Self::Context) -> Self::Result {
        let id = Uuid::new_v4().to_string();
        let (tx, rx) = oneshot::channel();

        self.get_futures.insert(id.clone(), tx);

        let req = InterfaceRequest(TaskQueryRequest::Get { id, query: msg.0 });

        Box::pin(
            self.writer
                .send(req)
                .then(|_res| async move { rx.await.unwrap() }),
        )
    }
}

impl Handler<GetOneQuery> for QueryClient {
    type Result = ResponseFuture<Option<Task>>;

    fn handle(&mut self, msg: GetOneQuery, _ctx: &mut Self::Context) -> Self::Result {
        let id = Uuid::new_v4().to_string();
        let (tx, rx) = oneshot::channel();

        self.get_one_futures.insert(id.clone(), tx);

        let req = InterfaceRequest(TaskQueryRequest::GetOne { id, query: msg.0 });

        Box::pin(
            self.writer
                .send(req)
                .then(|_res| async move { rx.await.unwrap() }),
        )
    }
}

impl StreamHandler<Result<TaskQueryResponse, Error>> for QueryClient {
    fn handle(&mut self, item: Result<TaskQueryResponse, Error>, _ctx: &mut Self::Context) {
        match item {
            Ok(TaskQueryResponse::Tasks { id, tasks }) => {
                if let Some(tx) = self.get_futures.remove(&id) {
                    tx.send(tasks).unwrap();
                }
            }
            Ok(TaskQueryResponse::Task { id, task }) => {
                if let Some(tx) = self.get_one_futures.remove(&id) {
                    tx.send(Some(task)).unwrap();
                }
            }
            Ok(TaskQueryResponse::NotFound { id }) => {
                if let Some(tx) = self.get_one_futures.remove(&id) {
                    tx.send(None).unwrap();
                }
            }
            Err(e) => {
                println!("Error: {:?}", e);
            }
        }
    }
}

impl QueryClient {
    #[allow(dead_code)]
    pub async fn connect(path: &str) -> Result<Addr<Self>, Error> {
        let stream = UnixStream::connect(path).await?;
        let (r, w) = tokio::io::split(stream);

        let writer = WriteInterface::attach(w).await?;

        let addr = QueryClient::create(|ctx| {
            QueryClient::listen(r, ctx);

            QueryClient {
                writer,
                get_futures: StdHash::new(),
                get_one_futures: StdHash::new(),
            }
        });

        Ok(addr)
    }
}
