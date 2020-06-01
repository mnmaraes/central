use actix::prelude::*;

use cliff::router;

struct Calculator {
    stack: Vec<f32>,
}

impl Actor for Calculator {
    type Context = Context<Self>;
}

router! {
    Calculator [
        Add { op1: f32, op2: f32 } -> {
            let result = op1 + op2;
        } => Result [f32] { result },
        Divide { op1: f32, op2: f32 } => [
            op2 != 0.0 => Result { result: op1 /  op2 },
            => Error [String, String] { operation: "Divide".into(), description: "Can't divide by zero".into() }
        ]
    ]
}

router! {
    Calculator;
    [
        Sci [
            Sin { n: f32 } => Result [f32] { result: n.sin() },
            Cos { n: f32 } => Result { result: n.cos() },
            Tan { n: f32 } => Result { result: n.tan() },
        ],
        Stack [
            Push { n: f32 } -> { self.stack.push(n); } => Success,
            Add -> {
                let op1 = self.stack.pop();
                let op2 = self.stack.pop();
            } => [
                op1 == None || op2 == None => Error [String, String] { operation: "Add".into(), description: "Not enough operands".into() },
                => Result [f32] { result: op1.expect("") + op2.expect("") }
            ],
            Mult -> {
                let op1 = self.stack.pop();
                let op2 = self.stack.pop();
            } => [
                op1 == None || op2 == None => Error { operation: "Mult".into(), description: "Not enough operands".into() },
                => Result { result: op1.expect("") * op2.expect("") }
            ]
        ]
    ]
}

fn main() {
    println!("Hello, world!");
}
