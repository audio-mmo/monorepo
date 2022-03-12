use ammo_ecs::*;

#[derive(Component, Clone, Debug, serde::Serialize, serde::Deserialize)]
#[ammo(
    namespace = "\"example\"",
    id = "\"foo\"",
    int_namespace = "2",
    int_id = "1"
)]
struct Foo {
    bar: String,
}

fn main() {
    let c = Foo { bar: "hi".into() };
    println!("{}", c.get_string_id());
    println!("{}", c.get_int_id());
}
