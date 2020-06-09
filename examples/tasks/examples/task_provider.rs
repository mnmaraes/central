use registry::run_provide;
use tasks::{TaskCommandRequest, TaskQueryRequest, TaskStore};

run_provide! {
    TaskStore => [TaskCommand, TaskQuery]
}
