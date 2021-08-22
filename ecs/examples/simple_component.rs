use ammo_ecs::component::*;
use ammo_ecs::*;

#[derive(Component, Clone, Debug, serde::Serialize, serde::Deserialize)]
#[ammo(namespace = "example", name = "foo", int_namespace = 2, int_id = 1)]
struct Foo {
    bar: String,
}

fn main() {
    let c = Foo { bar: "hi".into() };
    println!("{:?}", c.get_string_namespace());
    println!("{:?}", c.get_integral_namespace());
}
