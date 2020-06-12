#[macro_use]
extern crate diesel;

mod ipc;
mod models;
mod schema;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
