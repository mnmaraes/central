use actix::prelude::*;

use im::HashMap;

use serde::{Deserialize, Serialize};

use failure::Error;

use uuid::Uuid;

use cliff::server::IpcServer;
use cliff::{client, router};

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

#[derive(Debug, Default)]
pub struct TaskStore {
    tasks: HashMap<String, Task>,
}

impl Actor for TaskStore {
    type Context = Context<Self>;
}

router! {
    TaskStore;
    [
        TaskCommand [
            Create { name: String } -> {
                let task = Task::new(name);
                self.tasks.insert(task.id.clone(), task);
            } => Success,
            Complete { task_id: String } -> {
                let err_desc = match self.tasks.get_mut(&task_id) {
                    Some(task) if !task.complete => {
                        task.complete = true;
                        None
                    }
                    Some(_) => Some("Task Already Complete".to_string()),
                    None => Some("Task Not Found".to_string())
                };
            } => [
                let Some(description) = err_desc => Error [String] {description},
                => Success
            ]
        ],
        TaskQuery [
            GetOne { query: TaskQuery } -> {
                let task = self.tasks.values().find(|t| query.eval(t));
            } => [
                let Some(task) = task => Task [Task] { task: task.clone() },
                => NotFound
            ],
            Get { query: TaskQuery } -> {
                let tasks = self.tasks.values().filter(|t| query.eval(t)).cloned().collect();
            } => Tasks [Vec<Task>] { tasks }
        ]
    ]
}

pub fn serve_command(path: &str, addr: &Addr<TaskStore>) -> Result<(), Error> {
    IpcServer::<TaskCommandRequest, TaskStore>::serve(path, addr.clone())
}

pub fn serve_query(path: &str, addr: &Addr<TaskStore>) -> Result<(), Error> {
    IpcServer::<TaskQueryRequest, TaskStore>::serve(path, addr.clone())
}

client! {
    TaskCommand named Command {
        actions => [
            Create { name: String } wait,
            Complete { task_id: String } wait
        ],
        response_mapping => [
            Success => [ () ],
            Error { description: _ } => [ () ]
        ]
    }
}

client! {
    TaskQuery named Query {
        actions => [
            Get { query: TaskQuery } wait Vec<Task>,
            GetOne { query: TaskQuery } wait Option<Task>
        ],
        response_mapping => [
            Tasks { tasks } => [
                Vec<Task>: tasks
            ],
            Task { task } => [
                Option<Task>: Some(task)
            ],
            NotFound => [
                Option<Task>: None
            ]
        ]
    }
}
