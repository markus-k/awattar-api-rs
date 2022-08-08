use awattar_api::{query_prices, AwattarZone};

#[tokio::main]
async fn main() {
    let prices = query_prices(
        AwattarZone::Germany,
        Some(chrono::Local::now() - chrono::Duration::days(2)),
        Some(chrono::Local::now()),
    )
    .await
    .expect("Querying prices failed.");

    println!("Prices from two days ago to today:");
    for slot in prices {
        println!(
            "{} - {}: {:.02} €/kWh",
            slot.start,
            slot.end,
            slot.price_cents_per_kwh() as f32 / 100.00
        );
    }
}
