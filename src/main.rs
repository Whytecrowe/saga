use chrono::Local;
use saga_core::model::Echo;
use uuid::Uuid;

fn main() {
    println!("<=== Test ===>");

    // create an Echo
    let echo = Echo::new(
        Local::now().date_naive(),
        Uuid::new_v4(),
        "First Echo: Hello World!".to_string(),
    );

    println!("Echo created: {:#?}", echo);
    println!("Echo char count: {}", echo.char_count());

    // Test mutable borrow
    let mut echo2 = echo.clone();
    println!("Echo2 text initial: {:?}", echo2.markdown);
    echo2.update_markdown("Second Echo: Updated!".to_string());
    println!("Echo2 new text: {:?}", echo2.markdown);

    // Test consumption with conversion
    let markdown = echo2.into_markdown();
    println!("Echo2 consumed: {:?}", markdown);

    // date display
    println!("Echo display day: {}", echo.display_day());
}
