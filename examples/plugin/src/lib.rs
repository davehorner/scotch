use common::Object;

scotch_guest::export_alloc!();

#[cfg(target_arch = "wasm32")]
#[scotch_guest::host_functions]
extern "C" {
    fn print(val: &String);
    fn random_cat_fact() -> [String; 2];
}

// Stub implementations for native (non-wasm32) builds to satisfy linker and enable testing
#[cfg(not(target_arch = "wasm32"))]
fn print(val: &String) {
    // Native stub: simply print to stdout
    println!("{}", val);
}

#[cfg(not(target_arch = "wasm32"))]
fn random_cat_fact() -> [String; 2] {
    // Return placeholder cat facts
    [
        String::from("Cats sleep a lot."),
        String::from("Cats love boxes."),
    ]
}

#[scotch_guest::guest_function]
fn add_up_list(items: &Vec<i32>) -> i32 {
    // Print numbers in reverse because why not.
    #[cfg(target_arch = "wasm32")]
    items
        .iter()
        .rev()
        .map(|num| format!("Hello number, {num}"))
        .for_each(|text| print(&text));

    items.iter().sum::<i32>()
}

#[scotch_guest::guest_function]
fn sum_object(obj: &Object) -> f32 {
    obj.first + obj.second as f32
}

#[scotch_guest::guest_function]
fn greet(name: &String) -> String {
    let [fact1, fact2] = random_cat_fact();
    format!("Hello, {name}! Did you know that {fact1} and {fact2}")
}
